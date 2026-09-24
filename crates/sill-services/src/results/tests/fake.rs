//! A provider under the test's control: an immediate batch, then (after a delay, and only if
//! the query is still current unless told to ignore that) a second batch whose titles carry
//! the query text (`Late@fo`), so a test can tell which generation a late row came from.

use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use sill_launcher::{
    Activation, Batch, Choice, IconRef, Item, ItemId, Provider, ProviderKind, Query, ResultSink,
    Score,
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

/// What the fake does when queried.
#[derive(Debug)]
enum Behaviour {
    Answer,
    Panic,
}

#[derive(Debug)]
struct Script {
    kind: ProviderKind,
    first: Mutex<Vec<(String, u32)>>,
    delay: Duration,
    second: Vec<(String, u32)>,
    obeys: Obeys,
    behaviour: Behaviour,
    heard: Mutex<Vec<Heard>>,
}

/// A handle on one fake: build it, hand out [`Fake::provider`], read [`Fake::heard`].
#[derive(Debug, Clone)]
pub(super) struct Fake(Arc<Script>);

impl Fake {
    pub(super) fn new(kind: ProviderKind) -> Fake {
        Fake(Arc::new(Script {
            kind,
            first: Mutex::new(Vec::new()),
            delay: Duration::ZERO,
            second: Vec::new(),
            obeys: Obeys::Cancel,
            behaviour: Behaviour::Answer,
            heard: Mutex::new(Vec::new()),
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

    pub(super) fn then(self, delay: Duration, rows: &[(&str, u32)]) -> Fake {
        let rows = owned(rows);
        self.with(|s| {
            s.delay = delay;
            s.second = rows;
        })
    }

    pub(super) fn obeys(self, obeys: Obeys) -> Fake {
        self.with(|s| s.obeys = obeys)
    }

    pub(super) fn panics(self) -> Fake {
        self.with(|s| s.behaviour = Behaviour::Panic)
    }

    /// From now on the first batch is empty.
    pub(super) fn clear_first(&self) {
        lock(&self.0.first).clear();
    }

    pub(super) fn heard(&self) -> Vec<Heard> {
        lock(&self.0.heard).clone()
    }

    pub(super) fn provider(&self) -> Arc<dyn Provider> {
        Arc::new(self.clone())
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
        let first = lock(&script.first).clone();
        sink.push(batch(script.kind, query, &first, ""));
        if script.second.is_empty() {
            return;
        }
        std::thread::sleep(script.delay);
        if script.obeys == Obeys::Cancel && query.is_cancelled() {
            lock(&script.heard).push(Heard::Cancelled);
            return;
        }
        sink.push(batch(script.kind, query, &script.second, &query.text));
        lock(&script.heard).push(Heard::Finished);
    }

    fn activate(&self, _item: &Item, _choice: &Choice) -> Activation {
        Activation::Nothing
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
