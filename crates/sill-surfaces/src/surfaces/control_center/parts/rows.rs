//! Q79's stand-in: a settings-style list of networks or devices (a leading glyph, a title, a
//! detail, trailing marks; pressable), outside a floating menu. quire's `SettingsRow` (on
//! `cc-parts`) is that row; until it lands the rows are a quire `Menu` drawn in place
//! (`Flow::Inline`, `PickDismiss::Stay`), whose `MenuEntry::Row` has the glyph tile, the
//! detail, a check and a trailing glyph button, the nearest quire shape.

use dioxus::prelude::*;
use ds::{
    Anchor, Availability, Check, Flow, Icon, Menu, MenuEntrance, MenuEntry, MenuKind, MenuRow,
    PickDismiss, Point, Px, RowAction, Text, Tile,
};

/// What trails a row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum RowMark {
    Nothing,
    /// A secured network: a lock.
    Lock,
}

/// One row: what picking it yields, its glyph, title and detail (empty for none), a check when
/// it is the one in use, its trailing mark.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct SettingsRow<T> {
    pub value: T,
    pub glyph: Icon,
    pub title: String,
    pub detail: String,
    pub check: Check,
    pub mark: RowMark,
    pub availability: Availability,
}

/// The rows, in order; a pick calls `onpick` and the list stays.
#[component]
pub(crate) fn SettingsRows<T: Clone + PartialEq + 'static>(
    label: String,
    rows: Vec<SettingsRow<T>>,
    onpick: EventHandler<T>,
) -> Element {
    let entries: Vec<MenuEntry<T>> = rows.into_iter().map(entry).collect();
    rsx! {
        div { class: "sill-cc-rows", role: "group", "aria-label": "{label}",
            Menu {
                kind: MenuKind::Rich,
                anchor: Anchor::Point(Point { x: Px(0.0), y: Px(0.0) }),
                entries,
                onpick: move |value: T| onpick.call(value),
                onclose: move |()| {},
                entrance: MenuEntrance::Instant,
                dismiss: PickDismiss::Stay,
                flow: Flow::Inline,
            }
        }
    }
}

fn entry<T>(row: SettingsRow<T>) -> MenuEntry<T> {
    let trailing = match row.mark {
        RowMark::Nothing => None,
        RowMark::Lock => Some(RowAction {
            icon: Icon::Lock,
            label: "Secured".to_owned(),
            on_press: EventHandler::new(|_| {}),
        }),
    };
    let detail = (!row.detail.is_empty()).then(|| Text::from(row.detail));
    let check = match row.check {
        Check::Checked => Some(Check::Checked),
        Check::Unchecked => None,
    };
    MenuEntry::Row(MenuRow {
        detail,
        tile: Some(Tile::Icon(row.glyph)),
        check,
        availability: row.availability,
        trailing,
        ..MenuRow::new(row.value, row.title)
    })
}
