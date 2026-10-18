//! Arithmetic and unit conversion with fend (fend-core, 50 ms budget, Copy).

mod detect;
mod eval;
pub(crate) mod fetch;
pub mod rates;

use std::path::PathBuf;
use std::time::Duration;

use sill_launcher::{
    Action, ActionId, Activation, Batch, Choice, IconRef, Item, ItemId, Provider, ProviderKind,
    Query, ResultSink, Score,
};

use eval::{EvalOutcome, evaluate};
use rates::{Rates, SharedRates};
use tokio::sync::watch;

use crate::clock::unix_now;
use crate::providers::locked::copy_of;

use sill_model::results::{CurrencyPolicy, LauncherPolicy};

/// How long one evaluation may run before it is abandoned.
pub const CALCULATOR_BUDGET: Duration = Duration::from_millis(50);

/// The `ItemId` prefix a calculator result's id carries; `activate` strips it back off to
/// recover the exact value to copy, rather than re-parsing the row's display title.
const ID_PREFIX: &str = "calc:";

/// The id of the result row's secondary action: replaces the query with the result instead of
/// copying it (`Activation::Insert`).
const INSERT_ACTION: &str = "insert";

/// A calculator result is always the sole, most relevant row it offers.
const TOP_SCORE: Score = Score(1000);

/// One row, `= <result>`, when the query evaluates; choosing it copies the result. With
/// `launcher.currency` on, currency codes convert at `rates`.
#[derive(Debug, Clone)]
pub struct CalculatorProvider {
    pub policy: watch::Receiver<LauncherPolicy>,
    pub rates: SharedRates,
    /// The ECB rates cache ([`fetch::cache_path`] of the cache directory).
    pub rates_file: PathBuf,
}

impl Default for CalculatorProvider {
    /// Currency off for good (its policy's sender is gone), so no rates file is ever read:
    /// arithmetic and units only.
    fn default() -> Self {
        CalculatorProvider {
            policy: watch::channel(LauncherPolicy::default()).1,
            rates: SharedRates::default(),
            rates_file: PathBuf::new(),
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
        // sees while `launcher.currency = "ecb"`.
        let rates = if detect::mentions_possible_currency(&query.text) {
            self.ecb_rates()
        } else {
            None
        };
        let EvalOutcome::Value(value) = evaluate(&query.text, CALCULATOR_BUDGET, rates.as_ref())
        else {
            // `NoMatch` (not an expression, or fend rejected it) and `TimedOut` both mean no
            // item.
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
    /// The day's ECB rates while `launcher.currency = "ecb"`, else `None`: loads the cache fresh
    /// from disk (cheap; never the network) and, off this call's return path,
    /// starts a background fetch when the cache is due for a new one.
    fn ecb_rates(&self) -> Option<Rates> {
        if self.policy.borrow().currency != CurrencyPolicy::Ecb {
            return None;
        }
        let path = self.rates_file.clone();
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
            None => copy_of(&self.rates).into(),
        }
    }
}

/// The one row a successful evaluation yields: primary Enter copies it, the secondary "Insert"
/// action carries on calculating from it.
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
