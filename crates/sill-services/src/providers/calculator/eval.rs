//! Evaluating a query with fend-core inside [`CALCULATOR_BUDGET`]
//! (`crate::providers::calculator::CALCULATOR_BUDGET`).
//!
//! fend's own interrupt mechanism is cooperative (`Interrupt::should_interrupt` is polled
//! between steps of the evaluation, not a hard preemption), so "abort past it" means: hand fend
//! a deadline it checks itself, and read the clock again once it returns rather than pattern
//! match its error string (`CONVENTIONS.md` "substrings are not tokens" — fend's error text is
//! not a stable vocabulary we own).

use std::time::{Duration, Instant};

use fend_core::{Context, Interrupt, evaluate_with_interrupt};

use super::detect::looks_like_expression;
use super::rates::{ExchangeRates, Rates};

/// What evaluating a query produced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EvalOutcome {
    /// fend evaluated the query to this display string.
    Value(String),
    /// Not an expression, or fend rejected it (parse error, wrong number of arguments, ...).
    NoMatch,
    /// The budget ran out before fend produced a result.
    TimedOut,
}

/// A deadline fend polls cooperatively.
struct Deadline(Instant);

impl Interrupt for Deadline {
    fn should_interrupt(&self) -> bool {
        Instant::now() >= self.0
    }
}

/// Evaluates `text` with fend-core, but only when [`looks_like_expression`] says it is worth
/// trying, and never past `budget`. `rates` is the day's ECB table while
/// `launcher.currency = "ecb"` (F650); with `None`, currency codes are not units and no
/// exchange-rate handler is installed.
pub fn evaluate(text: &str, budget: Duration, rates: Option<&Rates>) -> EvalOutcome {
    if !looks_like_expression(text, rates) {
        return EvalOutcome::NoMatch;
    }
    let mut context = Context::new();
    if let Some(rates) = rates {
        context.set_exchange_rate_handler_v2(ExchangeRates::new(rates.clone()));
    }
    let deadline = Deadline(Instant::now() + budget);
    let started = Instant::now();
    match evaluate_with_interrupt(text, &mut context, &deadline) {
        Ok(result) => {
            let value = result.get_main_result();
            if value.is_empty() {
                EvalOutcome::NoMatch
            } else {
                EvalOutcome::Value(value.to_owned())
            }
        }
        Err(_) if started.elapsed() >= budget => EvalOutcome::TimedOut,
        Err(_) => EvalOutcome::NoMatch,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::time::{Duration, Instant};

    use super::{EvalOutcome, Rates, evaluate};

    #[test]
    fn plain_arithmetic_evaluates() {
        let budget = Duration::from_millis(50);
        assert_eq!(
            evaluate("2 + 2", budget, None),
            EvalOutcome::Value("4".to_owned())
        );
        assert_eq!(
            evaluate("2^10", budget, None),
            EvalOutcome::Value("1024".to_owned())
        );
    }

    #[test]
    fn a_unit_conversion_evaluates() {
        let outcome = evaluate("1 km to m", Duration::from_millis(50), None);
        assert_eq!(outcome, EvalOutcome::Value("1000 m".to_owned()));
    }

    #[test]
    fn a_plain_word_never_reaches_fend() {
        assert_eq!(
            evaluate("firefox", Duration::from_millis(50), None),
            EvalOutcome::NoMatch
        );
    }

    #[test]
    fn a_malformed_expression_is_no_match_not_a_panic() {
        assert_eq!(
            evaluate("2 + + +", Duration::from_millis(50), None),
            EvalOutcome::NoMatch
        );
    }

    /// A pathological expression (a large factorial): fend's `BigUint::factorial` checks the
    /// interrupt once per multiplication (fend-core `src/num/biguint.rs`), so this both proves
    /// the budget is enforced and stays well inside it — a wall-clock property, run once rather
    /// than table-driven.
    #[test]
    fn a_pathological_expression_is_abandoned_within_the_budget() {
        let budget = Duration::from_millis(50);
        let started = Instant::now();
        let outcome = evaluate("1000000!", budget, None);
        let elapsed = started.elapsed();
        assert_eq!(outcome, EvalOutcome::TimedOut, "{outcome:?}");
        assert!(
            elapsed < budget * 4,
            "abandoning a timed-out evaluation took {elapsed:?}, budget was {budget:?}"
        );
    }

    /// With `launcher.currency = "off"` (`rates: None`), a currency code is a plain search term:
    /// F650.
    #[test]
    fn without_rates_a_currency_code_never_reaches_fend() {
        assert_eq!(
            evaluate("10 usd to eur", Duration::from_millis(50), None),
            EvalOutcome::NoMatch
        );
    }

    /// With `launcher.currency = "ecb"` and a table of rates, fend converts through the
    /// exchange-rate handler `evaluate` installs.
    #[test]
    fn a_currency_conversion_uses_the_given_rates() {
        let rates = Rates {
            per_euro: BTreeMap::from([("USD".to_owned(), 1.1), ("EUR".to_owned(), 1.0)]),
            day: None,
        };
        let outcome = evaluate("10 usd to eur", Duration::from_millis(50), Some(&rates));
        let EvalOutcome::Value(value) = outcome else {
            panic!("expected a conversion, got {outcome:?}");
        };
        assert!(value.contains("eur") || value.contains("EUR"), "{value}");
    }
}
