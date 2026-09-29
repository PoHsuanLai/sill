//! The search task: one generation per keystroke, every provider on its own blocking task,
//! batches merged through `step` as they arrive, the merged list published in
//! Spotlight's sections with their spans (`arrange`).

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use sill_launcher::{
    Cancel, Choice, Counting, ItemKey, Provider, Query, Scope, SectionPlan, UnixTime,
};
use sill_model::results::{
    Asked, LauncherPolicy, Results, ResultsEvent, SearchCommand, Typed, arranged, step,
};
use tokio::sync::{mpsc, watch};

use super::effect::{Outlets, effect};
use super::history::{History, Persist};
use super::sink::{Arrival, ChannelSink, FinishedOnDrop};

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
    tx: watch::Sender<Results>,
    arrivals: mpsc::UnboundedSender<Arrival>,
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
            tx,
            arrivals,
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
        tokio::select! {
            biased;
            command = commands.recv() => match command {
                Some(command) => search.command(command),
                None => return,
            },
            // `search` holds a sender, so this never yields `None`.
            Some(arrival) = arrivals.recv() => search.arrive(arrival),
        }
    }
}

impl Search {
    pub(super) fn command(&mut self, command: SearchCommand) {
        match command {
            SearchCommand::Query(text) => self.query(text, Scope::All),
            SearchCommand::Filtered { text, provider } => self.query(text, Scope::Only(provider)),
            SearchCommand::Chosen { key, choice } => self.chosen(key, &choice),
            SearchCommand::Cancel => self.close(),
            SearchCommand::Policy(policy) => {
                self.policy.send_replace(policy);
            }
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
        self.publish();
    }

    /// Count the choice (whichever of the row's choices it was), queue the file write, and
    /// carry out what the row's provider says the choice does. The row is looked up among the
    /// newest generation's rows, so the surface sends `Chosen` before it closes (`Cancel`).
    fn chosen(&mut self, key: ItemKey, choice: &Choice) {
        // A clipboard entry's use is never written to disk.
        if key.counting() == Counting::Counted {
            let frecency = self.history.record(key.clone(), now());
            self.persist.save(frecency);
        }
        let Some(item) = self
            .merged
            .items
            .iter()
            .find(|item| ItemKey::of(item.provider, &item.id) == key)
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
                    self.publish();
                }
            }
            Arrival::Finished(_) => {}
        }
    }

    /// Publish the merged rows in their sections, only when they changed.
    fn publish(&self) {
        let placement = self
            .history
            .read(|frecency| arranged(&self.merged.items, &self.plan, self.asked, frecency, now()));
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

/// The wall clock, for frecency ages.
fn now() -> UnixTime {
    UnixTime(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |since| since.as_secs()),
    )
}
