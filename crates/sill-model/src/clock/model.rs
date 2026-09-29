//! The bar clock's text.

use serde::{Deserialize, Serialize};

/// The clock's text as the bar shows it: `Thu 24 Sep 14:05`.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ClockText {
    pub text: String,
}
