//! Arithmetic and unit conversion with fend (plan: "calculator (fend-core, 50 ms budget,
//! Copy)").

mod detect;
mod eval;
mod fetch;
pub mod rates;

use std::time::Duration;

use sill_launcher::{
    Action, ActionId, Activation, Batch, Choice, IconRef, Item, ItemId, Provider, ProviderKind,
    Query, ResultSink, Score, UnixTime,
};

use eval::{EvalOutcome, evaluate};
use rates::{Rates, SharedRates};
use tokio::sync::watch;

use crate::results::{CurrencyPolicy, LauncherPolicy};

/// How long one evaluation may run before it is abandoned.
pub const CALCULATOR_BUDGET: Duration = Duration::from_millis(50);

/// The `ItemId` prefix a calculator result's id carries; `activate` strips it back off to
/// recover the exact value to copy, rather than re-parsing the row's display title.
const ID_PREFIX: &str = "calc:";

/// The id of the result row's secondary action: replaces the query with the result instead of
/// copying it (`Activation::Insert`, FINDINGS F110/F116).
const INSERT_ACTION: &str = "insert";

/// A calculator result is always the sole, most relevant row it offers.
const TOP_SCORE: Score = Score(1000);

/// One row, `= <result>`, when the query evaluates; choosing it copies the result. With
/// `launcher.currency` on, currency codes convert at `rates` (M9 lane c, F650).
#[derive(Debug, Clone)]
pub struct CalculatorProvider {
    pub policy: watch::Receiver<LauncherPolicy>,
    pub rates: SharedRates,
}

impl Default for CalculatorProvider {
    /// Currency off, no rates: the M4 calculator.
    fn default() -> Self {
        CalculatorProvider {
            policy: watch::channel(LauncherPolicy::default()).1,
            rates: SharedRates::default(),
        }
    }
}

impl Provider for CalculatorProvider {
    fn kind(&self) -> ProviderKind {
        ProviderKind::Calculator
    }

    fn query(&self, query: &Query, sink: &mut dyn ResultSink) {
        if query.is_cancelled() {
            return;
        }
        // Loading the day's ECB rates is a disk read and a check whether a fetch is due; asked
        // only when the text could possibly need them, not for every keystroke the calculator
        // sees while `launcher.currency = "ecb"` (a coordinator follow-up on this lane).
        let rates = if detect::mentions_possible_currency(&query.text) {
            self.ecb_rates()
        } else {
            None
        };
        let EvalOutcome::Value(value) = evaluate(&query.text, CALCULATOR_BUDGET, rates.as_ref())
        else {
            // `NoMatch` (not an expression, or fend rejected it) and `TimedOut` both mean no
            // item: FINDINGS.md "M4 providers-b".
            return;
        };
        if query.is_cancelled() {
            return;
        }
        sink.push(Batch {
            provider: ProviderKind::Calculator,
            generation: query.generation,
            items: vec![result_item(&value)],
        });
    }

    fn activate(&self, item: &Item, choice: &Choice) -> Activation {
        let text = item
            .id
            .0
            .strip_prefix(ID_PREFIX)
            .unwrap_or(&item.id.0)
            .to_owned();
        match choice {
            Choice::Action(id) if id.0 == INSERT_ACTION => Activation::Insert { text },
            Choice::Primary | Choice::Action(_) => Activation::Copy { text },
        }
    }
}

impl CalculatorProvider {
    /// The day's ECB rates while `launcher.currency = "ecb"`, else `None` (F650): loads the
    /// cache fresh from disk (cheap; never the network) and, off this call's return path,
    /// starts a background fetch when the cache is due for a new one.
    fn ecb_rates(&self) -> Option<Rates> {
        if self.policy.borrow().currency != CurrencyPolicy::Ecb {
            return None;
        }
        let path = fetch::cache_path();
        fetch::refresh_if_due(self.rates.clone(), path.clone(), unix_now());
        match fetch::load(&path) {
            Some(loaded) => {
                if let Ok(mut guard) = self.rates.lock() {
                    *guard = loaded.clone();
                }
                Some(loaded)
            }
            // No cache yet (or a damaged one): whatever the shared rates already hold, from an
            // earlier fetch this process made.
            None => self
                .rates
                .lock()
                .map(|guard| guard.clone())
                .unwrap_or_else(|poisoned| poisoned.into_inner().clone())
                .into(),
        }
    }
}

/// The wall clock, in whole seconds since the epoch; a clock that cannot be read (never
/// observed, but not a reason to panic a keystroke) treats the cache as maximally stale, which
/// only means a fetch is attempted again.
fn unix_now() -> UnixTime {
    UnixTime(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0),
    )
}

/// The one row a successful evaluation yields: primary Enter copies it, the secondary "Insert"
/// action carries on calculating from it (FINDINGS F110/F116).
fn result_item(value: &str) -> Item {
    Item {
        id: ItemId(format!("{ID_PREFIX}{value}")),
        provider: ProviderKind::Calculator,
        title: format!("= {value}"),
        subtitle: String::new(),
        icon: IconRef::Glyph("calculator".to_owned()),
        score: TOP_SCORE,
        actions: vec![Action {
            id: ActionId(INSERT_ACTION.to_owned()),
            label: "Insert".to_owned(),
            keys: sill_launcher::ActionKeys::None,
        }],
        subject: sill_launcher::Subject::None,
    }
}

#[cfg(test)]
mod tests {
    use sill_launcher::{ActionId, Activation, Cancel, Choice, Item, Provider, Query, VecSink};

    use super::{CalculatorProvider, INSERT_ACTION};

    fn row(text: &str) -> Item {
        let provider = CalculatorProvider::default();
        let cancel = Cancel::new();
        let query = Query::begin(text, &cancel);
        let mut sink = VecSink::default();
        provider.query(&query, &mut sink);
        sink.0
            .into_iter()
            .flat_map(|batch| batch.items)
            .next()
            .unwrap_or_else(|| panic!("{text:?} should evaluate to one row"))
    }

    #[test]
    fn a_result_row_offers_an_insert_action_beside_the_primary_copy() {
        let item = row("2 + 2");
        assert_eq!(item.actions.len(), 1, "{:?}", item.actions);
        assert_eq!(item.actions[0].id.0, INSERT_ACTION);
        assert_eq!(item.actions[0].label, "Insert");
    }

    #[test]
    fn primary_copies_the_result_and_the_insert_action_replaces_the_query_with_it() {
        let provider = CalculatorProvider::default();
        let item = row("2 + 2");
        assert_eq!(
            provider.activate(&item, &Choice::Primary),
            Activation::Copy {
                text: "4".to_owned()
            }
        );
        assert_eq!(
            provider.activate(&item, &Choice::Action(ActionId(INSERT_ACTION.to_owned()))),
            Activation::Insert {
                text: "4".to_owned()
            }
        );
    }
}
