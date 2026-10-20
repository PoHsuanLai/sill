//! What the card shows, as data: the rows of a tab with what is picked marked, the list keys and
//! the words.

use ds::prelude::*;
use sill_model::capture_pick::{Picks, Plan, Plurality, Pointer, Remembered, Source, Tab};

/// One row, as data.
pub(super) struct RowData {
    pub(super) source: Source,
    pub(super) icon: Icon,
    pub(super) title: String,
    pub(super) detail: String,
    pub(super) selection: Selection,
    pub(super) check: Check,
}

/// The rows of `tab`, with what is picked marked.
pub(super) fn rows_of(plan: &Plan, tab: Tab, picks: &Picks) -> Vec<RowData> {
    let icon = match tab {
        Tab::Screens => Icon::Monitor,
        Tab::Windows => Icon::Window,
    };
    plan.on(tab)
        .iter()
        .map(|choice| {
            let (selection, check) = if picks.chosen.contains(&choice.source) {
                (Selection::Selected, Check::On)
            } else {
                (Selection::Unselected, Check::Off)
            };
            RowData {
                source: choice.source.clone(),
                icon,
                title: choice.title.clone(),
                detail: choice.detail.clone(),
                selection,
                check,
            }
        })
        .collect()
}

/// A list key for `source`: its kind and name.
pub(super) fn key_of(source: &Source) -> String {
    let (kind, name) = source.wire();
    format!("{kind}:{name}")
}

/// The source a list key names, when the plan still offers it.
pub(super) fn source_of(plan: &Plan, key: &str) -> Option<Source> {
    plan.screens
        .iter()
        .chain(&plan.windows)
        .map(|choice| &choice.source)
        .find(|source| key_of(source) == key)
        .cloned()
}

pub(super) fn tab_words(tab: Tab) -> &'static str {
    match tab {
        Tab::Screens => "Screens",
        Tab::Windows => "Windows",
    }
}

pub(super) fn empty_words(tab: Tab) -> &'static str {
    match tab {
        Tab::Screens => "No screens to share.",
        Tab::Windows => "No windows to share.",
    }
}

pub(super) fn pointer_words(pointer: Pointer) -> &'static str {
    match pointer {
        Pointer::Hidden => "Hidden",
        Pointer::Embedded => "Shown",
        Pointer::Metadata => "By the App",
    }
}

pub(super) fn byline_of(plan: &Plan, plurality: Plurality) -> &'static str {
    match (plan.tabs.len(), plurality) {
        (_, Plurality::Many) => "Pick one or more to share.",
        (1, Plurality::One) => "Pick one to share.",
        (_, Plurality::One) => "Pick a screen or a window to share.",
    }
}

/// The quiet sentence about how long the choice is kept; none when it is asked every time.
pub(super) fn remembered_words(remembered: Remembered, app: &str) -> Option<String> {
    match remembered {
        Remembered::EveryTime => None,
        Remembered::WhileRunning => Some(format!("Your choice is kept while {app} is open.")),
        Remembered::UntilTakenBack => {
            Some("Your choice is kept until you change it in Settings.".to_owned())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_remembered_mode_has_its_sentence() {
        assert_eq!(remembered_words(Remembered::EveryTime, "Meet"), None);
        assert_eq!(
            remembered_words(Remembered::WhileRunning, "Meet").as_deref(),
            Some("Your choice is kept while Meet is open.")
        );
        assert_eq!(
            remembered_words(Remembered::UntilTakenBack, "Meet").as_deref(),
            Some("Your choice is kept until you change it in Settings.")
        );
    }
}
