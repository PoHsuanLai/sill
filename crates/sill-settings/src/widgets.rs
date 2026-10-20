//! The widgets the notification center shows under its history: Calendar, then Up Next, always
//! (design/20 §1.14). Nothing places, orders or sizes them, so there is no setting here, only
//! the closed list of kinds the center draws.

use ds_core::word::Word;
use serde::{Deserialize, Serialize};

/// One widget of the notification center.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Word)]
#[serde(rename_all = "snake_case")]
pub enum WidgetKind {
    /// The month, today marked (macOS's Calendar widget, month size).
    Calendar,
    /// The next events from the calendar sources (macOS's Calendar "Up Next").
    UpNext,
}

#[cfg(test)]
mod tests {
    use super::WidgetKind;
    use ds_core::word::Word;

    #[test]
    fn slugs_are_the_kebab_case_element_id_suffixes() {
        let slugs: Vec<&str> = WidgetKind::ALL.iter().map(|kind| kind.slug()).collect();
        assert_eq!(slugs, ["calendar", "up-next"]);
    }
}
