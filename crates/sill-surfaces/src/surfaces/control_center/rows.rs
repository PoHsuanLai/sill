//! A detail pane's list (networks, devices) as quire `SettingsRow`s: each row's value, glyph,
//! words and trailing mark as data, so the pages build them purely and test them.

use dioxus::prelude::*;
use ds::{Availability, Icon, RowTrailing, SettingsRow, Switch, Text};

/// What a row ends in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum RowMark {
    Nothing,
    /// The network or device in use: a check.
    InUse,
    /// A secured network not in use: a lock.
    Lock,
}

/// One row: what choosing it yields, its glyph, title and detail (empty for none), its mark.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct RowSpec<T> {
    pub value: T,
    pub glyph: Icon,
    pub title: String,
    pub detail: String,
    pub mark: RowMark,
    pub availability: Availability,
}

/// The rows, in order, named `label`; choosing one calls `onpick` with its value.
#[component]
pub(crate) fn RowList<T: Clone + PartialEq + 'static>(
    label: String,
    rows: Vec<RowSpec<T>>,
    onpick: EventHandler<T>,
) -> Element {
    let items: Vec<(usize, RowSpec<T>, Option<Text>)> = rows
        .into_iter()
        .enumerate()
        .map(|(index, row)| {
            let detail = (!row.detail.is_empty()).then(|| Text::from(row.detail.clone()));
            (index, row, detail)
        })
        .collect();
    rsx! {
        div { class: "sill-cc-rows", role: "group", "aria-label": "{label}",
            for (index, row, detail) in items {
                SettingsRow {
                    key: "{index}",
                    glyph: Some(row.glyph),
                    title: row.title.clone(),
                    detail,
                    trailing: trailing(row.mark),
                    availability: row.availability,
                    onclick: {
                        let value = row.value.clone();
                        move |_| onpick.call(value.clone())
                    },
                }
            }
        }
    }
}

/// quire's trailing mark for `mark`.
fn trailing(mark: RowMark) -> RowTrailing {
    match mark {
        RowMark::Nothing => RowTrailing::None,
        RowMark::InUse => RowTrailing::Check(Switch::On),
        RowMark::Lock => RowTrailing::Glyph(Icon::Lock),
    }
}
