//! Arithmetic and unit conversion with fend (plan: "calculator (fend-core, 50 ms budget,
//! Copy)").

mod detect;
mod eval;

use std::time::Duration;

use sill_launcher::{
    Activation, Batch, Choice, IconRef, Item, ItemId, Provider, ProviderKind, Query, ResultSink,
    Score,
};

use eval::{EvalOutcome, evaluate};

/// How long one evaluation may run before it is abandoned.
pub const CALCULATOR_BUDGET: Duration = Duration::from_millis(50);

/// The `ItemId` prefix a calculator result's id carries; `activate` strips it back off to
/// recover the exact value to copy, rather than re-parsing the row's display title.
const ID_PREFIX: &str = "calc:";

/// A calculator result is always the sole, most relevant row it offers.
const TOP_SCORE: Score = Score(1000);

/// One row, `= <result>`, when the query evaluates; choosing it copies the result.
#[derive(Debug, Clone, Default)]
pub struct CalculatorProvider;

impl Provider for CalculatorProvider {
    fn kind(&self) -> ProviderKind {
        ProviderKind::Calculator
    }

    fn query(&self, query: &Query, sink: &mut dyn ResultSink) {
        if query.is_cancelled() {
            return;
        }
        let EvalOutcome::Value(value) = evaluate(&query.text, CALCULATOR_BUDGET) else {
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

    fn activate(&self, item: &Item, _choice: &Choice) -> Activation {
        let text = item
            .id
            .0
            .strip_prefix(ID_PREFIX)
            .unwrap_or(&item.id.0)
            .to_owned();
        Activation::Copy { text }
    }
}

/// The one row a successful evaluation yields.
fn result_item(value: &str) -> Item {
    Item {
        id: ItemId(format!("{ID_PREFIX}{value}")),
        provider: ProviderKind::Calculator,
        title: format!("= {value}"),
        subtitle: String::new(),
        icon: IconRef::Glyph("calculator".to_owned()),
        score: TOP_SCORE,
        // FINDINGS.md "M4 providers-b": the plan asks for a secondary "Insert" action, but the
        // frozen `Activation` vocabulary has no variant for "put this back in the search field"
        // — only `Copy{text}` carries text at all. Reported, not worked around with a new
        // frozen variant.
        actions: Vec::new(),
    }
}
