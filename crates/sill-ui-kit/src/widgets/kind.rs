//! The widgets the notification center shows under its history: Calendar, then Up Next, always
//! (design/20 §1.14). Nothing places, orders or sizes them, so there is no setting, only the
//! closed list the center draws.

use ds_core::word::Word;

/// One widget of the notification center.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Word)]
pub enum CenterWidget {
    /// The month, today marked (Calendar's month size).
    Calendar,
    /// The next events from the calendar sources (Calendar's "Up Next").
    UpNext,
}

#[cfg(test)]
mod tests {
    use ds_core::word::Word;

    use super::CenterWidget;

    #[test]
    fn slugs_are_the_kebab_case_element_id_suffixes() {
        let slugs: Vec<&str> = CenterWidget::ALL.iter().map(|kind| kind.slug()).collect();
        assert_eq!(slugs, ["calendar", "up-next"]);
    }
}
