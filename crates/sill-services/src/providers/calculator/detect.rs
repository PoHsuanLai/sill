//! Whether a query looks like something fend should evaluate, pure and table-tested: digits,
//! operators, units and known function names read as an expression; plain words never do,
//! whatever digits happen to appear in them (`"3 days ago"`, `"iPhone 15"`).
//!
//! ISO currency codes count as units only while `launcher.currency = "ecb"` has given a table
//! of them (M9 freeze F650): with it `off`, `rates` is `None` and `"10 usd to eur"` reads as a
//! plain search term, never an expression.

use super::rates::Rates;

/// fend's built-in function and operator names a bare word may spell.
const FUNCTIONS: &[&str] = &[
    "sqrt",
    "cbrt",
    "ln",
    "log",
    "log2",
    "log10",
    "sin",
    "cos",
    "tan",
    "asin",
    "acos",
    "atan",
    "abs",
    "floor",
    "ceil",
    "round",
    "exp",
    "factorial",
    "gcd",
    "lcm",
    "min",
    "max",
];

/// Conversion and expression keywords fend reads as bare words (`"5 km to miles"`).
const KEYWORDS: &[&str] = &["to", "in", "of", "as", "mod", "and", "or", "xor"];

/// Named constants fend evaluates on their own.
const CONSTANTS: &[&str] = &["pi", "e", "tau", "phi"];

/// Unit words fend recognises, standalone (`"5 km"`) or as a bare suffix on a number
/// (`"3m"`, `"10km"`).
const UNIT_WORDS: &[&str] = &[
    "km",
    "m",
    "cm",
    "mm",
    "mi",
    "miles",
    "mile",
    "ft",
    "feet",
    "foot",
    "in",
    "inch",
    "inches",
    "yd",
    "yard",
    "yards",
    "kg",
    "g",
    "mg",
    "lb",
    "lbs",
    "oz",
    "celsius",
    "fahrenheit",
    "kelvin",
    "c",
    "f",
    "k",
    "s",
    "sec",
    "secs",
    "seconds",
    "min",
    "mins",
    "minutes",
    "hr",
    "hrs",
    "hours",
    "day",
    "days",
    "kb",
    "mb",
    "gb",
    "tb",
    "bytes",
    "byte",
    "bit",
    "bits",
];

/// Operator and grouping characters that mark a string as arithmetic rather than a search term.
const SYMBOLS: &str = "+-*/^%()";

/// Whether `text` reads as an expression fend should try to evaluate: it has digits, an
/// operator, or an `=`, and every word in it is a number, a unit, a keyword or a known function
/// — never a plain search term such as `"cd"` or `"ls -la"`. `rates` is the day's ECB table
/// while `launcher.currency = "ecb"`, else `None` (F650): only then does a currency code count
/// as a unit word.
pub fn looks_like_expression(text: &str, rates: Option<&Rates>) -> bool {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return false;
    }
    if trimmed.contains('=') {
        return true;
    }
    let tokens = tokenize(trimmed);
    if tokens.is_empty() {
        return false;
    }
    let has_digit = tokens.iter().any(|t| t.chars().any(|c| c.is_ascii_digit()));
    let has_symbol = trimmed.chars().any(|c| SYMBOLS.contains(c));
    if !has_digit && !has_symbol {
        return false;
    }
    tokens.iter().all(|token| word_is_known(token, rates))
}

/// Splits on anything that is not alphanumeric or `.`, so operators, parens and whitespace all
/// separate words (`"sqrt(16)"` -> `["sqrt", "16"]`).
fn tokenize(text: &str) -> Vec<&str> {
    text.split(|c: char| !(c.is_alphanumeric() || c == '.'))
        .filter(|s| !s.is_empty())
        .collect()
}

/// Whether one token is a number, a number with a unit suffix (`"3m"`), or a bare word from the
/// known vocabulary (`rates`'s currency codes included, while there are any).
fn word_is_known(token: &str, rates: Option<&Rates>) -> bool {
    let digit_prefix = token
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '.')
        .count();
    let (number, rest) = token.split_at(digit_prefix);
    if rest.is_empty() {
        return true; // a bare number
    }
    let lower = rest.to_lowercase();
    if number.is_empty() {
        FUNCTIONS.contains(&lower.as_str())
            || KEYWORDS.contains(&lower.as_str())
            || CONSTANTS.contains(&lower.as_str())
            || UNIT_WORDS.contains(&lower.as_str())
            || is_currency(&lower, rates)
    } else {
        UNIT_WORDS.contains(&lower.as_str()) || is_currency(&lower, rates)
    }
}

/// Whether `lower` (already lower-cased) is one of `rates`'s currency codes.
fn is_currency(lower: &str, rates: Option<&Rates>) -> bool {
    rates.is_some_and(|rates| rates.per_euro.contains_key(&lower.to_uppercase()))
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::{Rates, looks_like_expression};

    const CASES: &[(&str, bool)] = &[
        // Plain numbers and arithmetic.
        ("42", true),
        ("3.14", true),
        ("2+2", true),
        ("2 + 2", true),
        ("2^10", true),
        ("(1+2)*3", true),
        ("50% of 200", true),
        ("sqrt(16)", true),
        ("sqrt (16)", true),
        ("1 + sqrt(9)", true),
        ("e^2", true),
        ("x = 5", true),
        ("5=5", true),
        // Unit conversions.
        ("5 km to miles", true),
        ("10 in cm", true),
        ("3m to ft", true),
        ("100 kg in lb", true),
        ("98.6 fahrenheit to celsius", true),
        // Plain words: no digits, no operators, not in the vocabulary.
        ("cd", false),
        ("ls -la", false),
        ("firefox", false),
        ("settings", false),
        ("", false),
        ("   ", false),
        // Words that happen to contain digits or hyphens but are not expressions.
        ("3 days ago", false),
        ("iPhone 15", false),
        ("iPhone 15 Pro", false),
        ("Chapter 7", false),
        ("cosmic-settings", false),
        ("rfc-2119", false),
    ];

    #[test]
    fn a_query_is_read_as_an_expression_exactly_when_it_looks_like_one() {
        for &(text, want) in CASES {
            assert_eq!(
                looks_like_expression(text, None),
                want,
                "looks_like_expression({text:?}, None) should be {want}"
            );
        }
    }

    fn sample_rates() -> Rates {
        Rates {
            per_euro: BTreeMap::from([("USD".to_owned(), 1.1), ("EUR".to_owned(), 1.0)]),
            day: None,
        }
    }

    #[test]
    fn a_currency_code_is_a_unit_only_while_ecb_gave_a_table_of_them() {
        assert!(!looks_like_expression("10 usd to eur", None));
        assert!(looks_like_expression(
            "10 usd to eur",
            Some(&sample_rates())
        ));
        // A code the table does not have is still a plain search term.
        assert!(!looks_like_expression(
            "10 twd to eur",
            Some(&sample_rates())
        ));
        // Case does not matter, as for every other unit word.
        assert!(looks_like_expression(
            "10 USD to EUR",
            Some(&sample_rates())
        ));
    }
}
