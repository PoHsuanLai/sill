//! A provider under the test's control: an immediate batch, then (once the test releases it,
//! and only if the query is still current unless told to ignore that) a second batch whose
//! titles carry the query text (`Late@fo`), so a test can tell which generation a late row came
//! from. The second batch waits on a gate the test opens, never on the clock, so what a test
//! observes does not depend on how fast the machine runs.

use std::sync::{Arc, Condvar, Mutex, PoisonError};
use std::time::Duration;

use tokio::sync::watch;

use sill_launcher::{
    Activation, AppId, Batch, Choice, DesktopAction, IconRef, Item, ItemId, Provider, ProviderKind,
    Query, ResultSink, Score, WindowKey,
};

/// Whether the fake stops when its query is overtaken.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Obeys {
    Cancel,
    Never,
}

/// What one query of the fake ended with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Heard {
    /// It saw the query cancelled before its second batch and stopped.
    Cancelled,
    /// It ran to the end.
    Finished,
}

/// What a fake has been asked and has answered so far, in the order it happened.
#[derive(Debug, Clone, Default)]
pub(super) struct Log {
    /// The text of every query its `query` has started on, in order.
    pub(super) asked: Vec<String>,
    /// How each query with a second batch ended, by query text.
    pub(super) heard: Vec<(String, Heard)>,
}

/// How long a blocked fake waits for its gate before it gives up. Only a hang guard for a test
/// that forgot to release; a passing test never waits on it.
const HANG_GUARD: Duration = Duration::from_secs(30);

/// What the fake does when queried.
#[derive(Debug)]
enum Behaviour {
    Answer,
    Panic,
    PanicOnActivate,
}

#[derive(Debug)]
struct Script {
    kind: ProviderKind,
    first: Mutex<Vec<(String, u32)>>,
    second: Vec<(String, u32)>,
    obeys: Obeys,
    behaviour: Behaviour,
    /// Open once the test releases the second batch.
    gate: (Mutex<bool>, Condvar),
    log: watch::Sender<Log>,
}

/// A handle on one fake: build it, hand out [`Fake::provider`], read [`Fake::heard`].
#[derive(Debug, Clone)]
pub(super) struct Fake(Arc<Script>);

impl Fake {
    pub(super) fn new(kind: ProviderKind) -> Fake {
        Fake(Arc::new(Script {
            kind,
            first: Mutex::new(Vec::new()),
            second: Vec::new(),
            obeys: Obeys::Cancel,
            behaviour: Behaviour::Answer,
            gate: (Mutex::new(false), Condvar::new()),
            log: watch::channel(Log::default()).0,
        }))
    }

    fn with(self, change: impl FnOnce(&mut Script)) -> Fake {
        let mut script = Arc::try_unwrap(self.0).unwrap_or_else(|_| panic!("built once"));
        change(&mut script);
        Fake(Arc::new(script))
    }

    pub(super) fn first(self, rows: &[(&str, u32)]) -> Fake {
        let rows = owned(rows);
        self.with(|s| s.first = Mutex::new(rows))
    }

    /// A second batch, held back until [`Fake::release`].
    pub(super) fn then(self, rows: &[(&str, u32)]) -> Fake {
        let rows = owned(rows);
        self.with(|s| s.second = rows)
    }

    pub(super) fn obeys(self, obeys: Obeys) -> Fake {
        self.with(|s| s.obeys = obeys)
    }

    pub(super) fn panics(self) -> Fake {
        self.with(|s| s.behaviour = Behaviour::Panic)
    }

    pub(super) fn panics_on_activate(self) -> Fake {
        self.with(|s| s.behaviour = Behaviour::PanicOnActivate)
    }

    /// From now on the first batch is empty.
    pub(super) fn clear_first(&self) {
        lock(&self.0.first).clear();
    }

    /// Let every query's second batch go, the ones waiting now and any later one.
    pub(super) fn release(&self) {
        *lock(&self.0.gate.0) = true;
        self.0.gate.1.notify_all();
    }

    /// How each query ended, ordered by query text (the order the threads finish in is the
    /// machine's, not the test's).
    pub(super) fn heard(&self) -> Vec<(String, Heard)> {
        let mut heard = self.0.log.borrow().heard.clone();
        heard.sort_by(|a, b| a.0.cmp(&b.0));
        heard
    }

    /// The first moment the log satisfies `ready`.
    pub(super) async fn wait(&self, ready: impl FnMut(&Log) -> bool) {
        let mut log = self.0.log.subscribe();
        let seen = tokio::time::timeout(super::PATIENCE, log.wait_for(ready)).await;
        if seen.is_err() {
            panic!("timed out; log {:?}", self.0.log.borrow());
        }
    }

    pub(super) fn provider(&self) -> Arc<dyn Provider> {
        Arc::new(self.clone())
    }
}

impl Script {
    fn wait_for_release(&self) {
        let (open, opened) = &self.gate;
        let _ = opened
            .wait_timeout_while(lock(open), HANG_GUARD, |open| !*open)
            .unwrap_or_else(PoisonError::into_inner);
    }
}

impl Provider for Fake {
    fn kind(&self) -> ProviderKind {
        self.0.kind
    }

    fn query(&self, query: &Query, sink: &mut dyn ResultSink) {
        let script = &self.0;
        if let Behaviour::Panic = script.behaviour {
            panic!("the fake provider panics, as told");
        }
        script
            .log
            .send_modify(|log| log.asked.push(query.text.clone()));
        let first = lock(&script.first).clone();
        sink.push(batch(script.kind, query, &first, ""));
        if script.second.is_empty() {
            return;
        }
        script.wait_for_release();
        let heard = |how| {
            script
                .log
                .send_modify(|log| log.heard.push((query.text.clone(), how)));
        };
        if script.obeys == Obeys::Cancel && query.is_cancelled() {
            heard(Heard::Cancelled);
            return;
        }
        sink.push(batch(script.kind, query, &script.second, &query.text));
        heard(Heard::Finished);
    }

    /// A window row (id = its key) focuses on Enter and closes from its action; an app row
    /// launches; anything else does nothing.
    fn activate(&self, item: &Item, choice: &Choice) -> Activation {
        if let Behaviour::PanicOnActivate = self.0.behaviour {
            panic!("the fake provider panics activating, as told");
        }
        match (self.0.kind, choice) {
            (ProviderKind::Windows, choice) => {
                let window = WindowKey(item.id.0.parse().unwrap_or_default());
                match choice {
                    Choice::Primary => Activation::FocusWindow { window },
                    Choice::Action(_) => Activation::CloseWindow { window },
                }
            }
            (ProviderKind::Apps, _) => Activation::Launch {
                app: AppId(item.id.0.clone()),
                action: DesktopAction::Default,
            },
            _ => Activation::Nothing,
        }
    }
}

fn batch(kind: ProviderKind, query: &Query, rows: &[(String, u32)], tag: &str) -> Batch {
    let items = rows
        .iter()
        .map(|(title, score)| {
            let title = if tag.is_empty() {
                title.clone()
            } else {
                format!("{title}@{tag}")
            };
            Item {
                id: ItemId(title.clone()),
                provider: kind,
                title,
                subtitle: String::new(),
                icon: IconRef::None,
                score: Score(*score),
                actions: Vec::new(),
                subject: sill_launcher::Subject::None,
            }
        })
        .collect();
    Batch {
        provider: kind,
        generation: query.generation,
        items,
    }
}

fn owned(rows: &[(&str, u32)]) -> Vec<(String, u32)> {
    rows.iter().map(|&(t, s)| (t.to_owned(), s)).collect()
}

fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}
