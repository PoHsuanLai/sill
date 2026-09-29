//! A detail pane's list (networks, devices) as quire `SettingsRow`s: each row's value, glyph,
//! words, trailing mark, operation phase and disc as data, so the pages build them purely and
//! test them. Rows are keyed by their item (an SSID, an address), so a row's detail state stays
//! with its item when the list reorders.

use dioxus::prelude::*;
use ds::detail::FirstShow;
use ds::{
    Availability, Fraction, Icon, RowDisc, RowPhase, RowTrailing, RowWork, SettingsRow, TextLine,
};
use sill_model::units::Percent;

/// What a row ends in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum RowMark {
    Nothing,
    /// A secured network: a lock (a joining row's spinner takes its place).
    Lock,
    /// A connected device's battery (design/26 G21).
    Battery(Percent),
}

/// One row: its key, what choosing it yields, its glyph, title and detail (empty for none), its
/// mark, the operation on its item, where that operation's loop shows, and its disc.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct RowSpec<T> {
    pub key: String,
    pub value: T,
    pub glyph: Icon,
    pub title: String,
    pub detail: String,
    pub mark: RowMark,
    pub phase: RowPhase,
    pub work: RowWork,
    pub disc: RowDisc,
    pub availability: Availability,
}

/// The rows, in order, named `label`; choosing one calls `onpick` with its value. `first` is
/// `Animate` on the pane's first frame (a battery sweeps in), `Still` after.
#[component]
pub(crate) fn RowList<T: Clone + PartialEq + 'static>(
    label: String,
    rows: Vec<RowSpec<T>>,
    first: FirstShow,
    onpick: EventHandler<T>,
) -> Element {
    let items: Vec<(RowSpec<T>, Option<TextLine>)> = rows
        .into_iter()
        .map(|row| {
            let detail = (!row.detail.is_empty()).then(|| TextLine::from(row.detail.clone()));
            (row, detail)
        })
        .collect();
    rsx! {
        div { class: "sill-cc-rows", role: "group", "aria-label": "{label}",
            for (row, detail) in items {
                SettingsRow {
                    key: "{row.key}",
                    glyph: Some(row.glyph),
                    title: row.title.clone(),
                    detail,
                    trailing: trailing(row.mark),
                    phase: row.phase,
                    work: row.work,
                    disc: row.disc,
                    first,
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
        RowMark::Lock => RowTrailing::Glyph(Icon::Lock),
        RowMark::Battery(level) => RowTrailing::Battery(Fraction(level.0.min(100) * 10)),
    }
}

/// `Animate` on a pane's first frame, `Still` on every later one: the pane was just opened
/// (a chevron, a bar item), so its rows' batteries sweep in once (R1).
pub(crate) fn use_first_show() -> FirstShow {
    let mut seen = use_hook(|| CopyValue::new(FirstShow::Animate));
    let now = *seen.peek();
    if now == FirstShow::Animate {
        seen.set(FirstShow::Still);
    }
    now
}
