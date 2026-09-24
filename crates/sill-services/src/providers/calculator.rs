//! Arithmetic and unit conversion with fend (plan: "calculator (fend-core, 50 ms budget,
//! Copy)").

#![allow(unused_variables, dead_code)] // Freeze stubs

use std::time::Duration;

use sill_launcher::{Activation, Choice, Item, Provider, ProviderKind, Query, ResultSink};

/// How long one evaluation may run before it is abandoned.
pub const CALCULATOR_BUDGET: Duration = Duration::from_millis(50);

/// One row, `= <result>`, when the query evaluates; choosing it copies the result.
#[derive(Debug, Clone, Default)]
pub struct CalculatorProvider;

impl Provider for CalculatorProvider {
    fn kind(&self) -> ProviderKind {
        ProviderKind::Calculator
    }

    fn query(&self, query: &Query, sink: &mut dyn ResultSink) {
        todo!("M4 providers-b: fend_core::evaluate_with_interrupt under CALCULATOR_BUDGET")
    }

    fn activate(&self, item: &Item, choice: &Choice) -> Activation {
        todo!("M4 providers-b: Copy{{text}}")
    }
}
