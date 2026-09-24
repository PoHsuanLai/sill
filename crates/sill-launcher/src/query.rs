//! A query and its generation: every keystroke starts a new generation, and a provider (or the
//! results merger) drops anything from an older one.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Serialize};

/// Which keystroke a query or a batch belongs to. Later is larger; 0 is before any keystroke.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default, Serialize, Deserialize,
)]
#[serde(transparent)]
pub struct Generation(pub u64);

/// The shared "current generation" counter. The launcher bumps it on every keystroke; a
/// provider holding an older [`Query`] sees it is stale and stops early.
#[derive(Debug, Clone, Default)]
pub struct Cancel(Arc<AtomicU64>);

impl Cancel {
    /// A fresh counter at generation 0.
    pub fn new() -> Cancel {
        Cancel::default()
    }

    /// Start a new generation and return it; every query of an earlier one is now stale.
    pub fn begin(&self) -> Generation {
        Generation(self.0.fetch_add(1, Ordering::AcqRel) + 1)
    }

    /// The newest generation begun.
    pub fn current(&self) -> Generation {
        Generation(self.0.load(Ordering::Acquire))
    }

    /// Whether `generation` has been overtaken by a newer one.
    pub fn is_stale(&self, generation: Generation) -> bool {
        generation < self.current()
    }
}

/// What the user typed, stamped with its generation.
#[derive(Debug, Clone)]
pub struct Query {
    /// The text as typed (operators and case intact; each provider parses what it needs).
    pub text: String,
    pub generation: Generation,
    pub cancel: Cancel,
}

impl Query {
    /// A query for `text` in a new generation of `cancel`.
    pub fn begin(text: impl Into<String>, cancel: &Cancel) -> Query {
        Query {
            text: text.into(),
            generation: cancel.begin(),
            cancel: cancel.clone(),
        }
    }

    /// Whether a newer keystroke has made this query pointless.
    pub fn is_cancelled(&self) -> bool {
        self.cancel.is_stale(self.generation)
    }
}

#[cfg(test)]
mod tests {
    use super::{Cancel, Generation, Query};

    #[test]
    fn a_newer_keystroke_cancels_every_older_query() {
        let cancel = Cancel::new();
        assert_eq!(cancel.current(), Generation(0));
        let first = Query::begin("f", &cancel);
        let second = Query::begin("fi", &cancel);
        assert_eq!(first.generation, Generation(1));
        assert_eq!(second.generation, Generation(2));
        assert!(first.is_cancelled(), "overtaken by `fi`");
        assert!(!second.is_cancelled(), "the newest is live");
        let third = Query::begin("fir", &cancel);
        assert!(second.is_cancelled());
        assert!(!third.is_cancelled());
    }
}
