//! The search task: one generation per keystroke, every provider on its own blocking task,
//! batches merged through `step` as they arrive, the merged list published in
//! Spotlight's sections with their spans (`arrange`).

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;
use std::time::Duration;

use sill_launcher::{Cancel, Choice, Counting, ItemKey, Provider, Query, Scope, SectionPlan};
use sill_model::apps::AppIndex;
use sill_model::results::{
    Asked, LauncherPolicy, Results, ResultsEvent, SearchCommand, Typed, arranged, step,
};
use sill_model::seed::{Store, first_frecency};
use tokio::sync::{mpsc, watch};
use tokio::time::Instant;

use super::effect::{Outlets, effect};
use super::history::{History, Persist};
use super::sink::{Arrival, ChannelSink, FinishedOnDrop};
use crate::clock::unix_now;

/// How long a generation waits for its slowest provider. Past it the providers still running are
/// no longer waited for: the rows that did arrive stand, an empty answer is shown as empty, and
/// whatever a straggler sends later is dropped with its generation. A provider stuck in a call
/// to a daemon or a stalled mount holds its thread, not the list.
pub(crate) const PROVIDER_DEADLINE: Duration = Duration::from_secs(3);

/// Everything the task holds between events.
pub(crate) struct Search {
    providers: Vec<Arc<dyn Provider>>,
    history: History,
    persist: Persist,
    outlets: Outlets,
    /// Where the settings-gated providers read the settings' policy from.
    policy: watch::Sender<LauncherPolicy>,
    cancel: Cancel,
    /// Every row of the newest generation, best first (`step`'s order).
    merged: Results,
    /// What the newest generation asked: its text's shape and scope.
    asked: Asked,
    /// Sections, weights and caps.
    plan: SectionPlan,
    /// Providers still running the newest generation.
    pending: usize,
    /// When the newest generation stops waiting for them; none when none is running.
    deadline: Option<Instant>,
    tx: watch::Sender<Results>,
    arrivals: mpsc::UnboundedSender<Arrival>,
    /// The installed apps, while a fresh account's history waits to start from them.
    pub(super) first_run: Option<watch::Receiver<AppIndex>>,
}

impl Search {
    /// A search over `providers` publishing to `tx`, and the channel its providers answer on.
    pub(crate) fn new(
        providers: Vec<Arc<dyn Provider>>,
        history: History,
        persist: Persist,
        outlets: Outlets,
        policy: watch::Sender<LauncherPolicy>,
        tx: watch::Sender<Results>,
    ) -> (Search, mpsc::UnboundedReceiver<Arrival>) {
        let (arrivals, rx) = mpsc::unbounded_channel();
        let search = Search {
            providers,
            history,
            persist,
            outlets,
            policy,
            cancel: Cancel::new(),
            merged: Results::default(),
            asked: Asked::IDLE,
            plan: SectionPlan::default_plan(),
            pending: 0,
            deadline: None,
            tx,
            arrivals,
            first_run: None,
        };
        (search, rx)
    }
}

/// Run until the `Control` is dropped. Commands go first, so a keystroke waiting behind a burst
/// of batches cancels them rather than queueing after them.
pub(crate) async fn run(
    mut search: Search,
    mut commands: mpsc::UnboundedReceiver<SearchCommand>,
    mut arrivals: mpsc::UnboundedReceiver<Arrival>,
) {
    loop {
        let deadline = search.deadline;
        tokio::select! {
            biased;
            command = commands.recv() => match command {
                Some(command) => search.command(command),
                None => return,
            },
            // `search` holds a sender, so this never yields `None`.
            Some(arrival) = arrivals.recv() => search.arrive(arrival),
            Some(installed) = installed(&mut search.first_run) => search.start_history(&installed),
            () = until(deadline) => search.give_up_waiting(),
        }
    }
}

/// Resolves at `deadline`; never when there is none.
async fn until(deadline: Option<Instant>) {
    match deadline {
        Some(at) => tokio::time::sleep_until(at).await,
        None => std::future::pending().await,
    }
}

/// The installed apps once there are any, while a fresh account's history waits for them.
async fn installed(apps: &mut Option<watch::Receiver<AppIndex>>) -> Option<AppIndex> {
    match apps {
        Some(apps) => apps
            .wait_for(|index| !index.apps.is_empty())
            .await
            .ok()
            .map(|index| AppIndex::clone(&index)),
        None => std::future::pending().await,
    }
}

impl Search {
    pub(super) fn command(&mut self, command: SearchCommand) {
        match command {
            SearchCommand::Query(text) => self.query(text, Scope::All),
            SearchCommand::Filtered { text, provider } => self.query(text, Scope::Only(provider)),
            SearchCommand::Chosen { key, choice } => self.chosen(&key, &choice),
            SearchCommand::Cancel => self.close(),
            SearchCommand::Policy(policy) => {
                self.policy.send_replace(policy);
            }
        }
    }

    /// Start a fresh account's history from the desktop's own apps, unless something was counted
    /// in the meantime (`sill_model::seed::first_frecency`).
    pub(super) fn start_history(&mut self, installed: &AppIndex) {
        self.first_run = None;
        let store = if self.history.untouched() {
            Store::Missing
        } else {
            Store::Present
        };
        if let Some(seed) = first_frecency(store, installed, unix_now()) {
            let frecency = self.history.start_from(seed);
            self.persist.save(frecency);
        }
    }

    /// Begin a generation (every older query is now cancelled) and start every provider on it.
    fn query(&mut self, text: String, scope: Scope) {
        self.asked = Asked {
            typed: Typed::of(&text),
            scope,
        };
        let query = Query {
            scope,
            ..Query::begin(text, &self.cancel)
        };
        let asked: Vec<Arc<dyn Provider>> = self
            .providers
            .iter()
            .filter(|p| match scope {
                Scope::All => true,
                Scope::Only(kind) => p.kind() == kind,
            })
            .cloned()
            .collect();
        self.merged = step(
            std::mem::take(&mut self.merged),
            ResultsEvent::Began(query.generation),
        );
        self.pending = asked.len();
        self.deadline = (self.pending > 0).then(|| Instant::now() + PROVIDER_DEADLINE);
        // The previous rows stay up until this generation has something to show (or every
        // provider has finished with nothing), so typing does not flash an empty list.
        if self.pending == 0 {
            self.publish();
        }
        for provider in asked {
            start(provider, query.clone(), self.arrivals.clone());
        }
    }

    /// The launcher closed: cancel the running generation and show nothing.
    fn close(&mut self) {
        let generation = self.cancel.begin();
        self.merged = step(
            std::mem::take(&mut self.merged),
            ResultsEvent::Began(generation),
        );
        self.asked = Asked::IDLE;
        self.pending = 0;
        self.deadline = None;
        self.publish();
    }

    /// The newest generation's deadline passed with providers still running: stop waiting for
    /// them and show what there is.
    pub(super) fn give_up_waiting(&mut self) {
        self.deadline = None;
        if self.pending > 0 {
            eprintln!(
                "sill: search: {} provider(s) did not answer within {PROVIDER_DEADLINE:?}",
                self.pending
            );
            self.pending = 0;
            self.publish();
        }
    }

    /// Count the choice (whichever of the row's choices it was), queue the file write, and
    /// carry out what the row's provider says the choice does. The row is looked up among the
    /// newest generation's rows, so the surface sends `Chosen` before it closes (`Cancel`).
    fn chosen(&self, key: &ItemKey, choice: &Choice) {
        // A clipboard entry's use is never written to disk.
        if key.counting() == Counting::Counted {
            let frecency = self.history.record(key.clone(), unix_now());
            self.persist.save(frecency);
        }
        let Some(item) = self
            .merged
            .items
            .iter()
            .find(|item| ItemKey::of(item.provider, &item.id) == *key)
        else {
            return;
        };
        let Some(provider) = self.providers.iter().find(|p| p.kind() == item.provider) else {
            return;
        };
        // A provider's `activate` is its own code: a panic there must not end the search.
        match catch_unwind(AssertUnwindSafe(|| provider.activate(item, choice))) {
            Ok(activation) => self.outlets.carry(effect(&activation)),
            Err(_) => eprintln!(
                "sill: the {:?} provider panicked activating a row",
                item.provider
            ),
        }
    }

    pub(super) fn arrive(&mut self, arrival: Arrival) {
        match arrival {
            Arrival::Batch(batch) => {
                // Late rows of an overtaken generation: `step` would drop them too,
                // but they must not cost a publish either.
                if batch.generation != self.merged.generation {
                    return;
                }
                if batch.items.is_empty() {
                    return;
                }
                self.merged = step(std::mem::take(&mut self.merged), ResultsEvent::Batch(batch));
                self.publish();
            }
            Arrival::Finished(generation) if generation == self.merged.generation => {
                self.pending = self.pending.saturating_sub(1);
                if self.pending == 0 {
                    self.deadline = None;
                    self.publish();
                }
            }
            Arrival::Finished(_) => {}
        }
    }

    /// Publish the merged rows in their sections, only when they changed.
    fn publish(&self) {
        let placement = self.history.read(|frecency| {
            arranged(
                &self.merged.items,
                &self.plan,
                self.asked,
                frecency,
                unix_now(),
            )
        });
        let next = Results {
            generation: self.merged.generation,
            items: placement.items,
            sections: placement.spans,
        };
        self.tx.send_if_modified(|shown| {
            let changed = *shown != next;
            if changed {
                *shown = next;
            }
            changed
        });
    }
}

/// Run `provider` on `query` on the blocking pool, answering on `arrivals`.
fn start(provider: Arc<dyn Provider>, query: Query, arrivals: mpsc::UnboundedSender<Arrival>) {
    tokio::task::spawn_blocking(move || {
        let _finished = FinishedOnDrop {
            generation: query.generation,
            tx: arrivals.clone(),
        };
        if query.is_cancelled() {
            return;
        }
        let mut sink = ChannelSink {
            provider: provider.kind(),
            query: &query,
            tx: &arrivals,
        };
        provider.query(&query, &mut sink);
    });
}
