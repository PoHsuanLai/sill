//! One row of the center's list, as quire's `List` takes it: a group's
//! header (quire's `GroupHeader`: the app's icon and name, the count, "N more" or "Show less",
//! Clear) keyed by its `AppKey`, or a card (quire's `NotificationCard` through [`NoteCard`], the
//! newest stacked over the rest while folded) keyed by its `NotificationId`. The keyboard's
//! ring is quire's: the header or card it is on carries `data-focus=ring`. The gap between rows
//! is the row's own padding (`.sill-nc-item`), never the list's.

use dioxus::prelude::*;
use ds::{Common, DataAttr, DataName, IconSize, ListItem};
use ds_shell::GroupHeader;
use sill_model::notifications::{AppGroup, History, Notification, UnixMs};

use super::groups::Expanded;
use super::io::CenterCtx;
use super::keys::RowFocus;
use super::rows::{CenterRow, RowAct, RowKey, press};
use sill_ui_kit::notification_card::{CardPlace, CardPress, NoteCard, use_app_mark};

/// Row `at` of the column, `row`, as a list item; `None` when the history no longer holds it.
pub(super) fn row_item(
    history: &History,
    expanded: &Expanded,
    (at, row): (usize, &CenterRow),
    focus: RowFocus,
    now: UnixMs,
) -> Option<ListItem<RowKey>> {
    let ring = ringed(focus, at);
    let group = history
        .groups
        .iter()
        .find(|group| group.key == *row.group())?;
    let content = match row {
        CenterRow::Head { .. } => rsx! {
            HeadRow { group: group.clone(), expanded: opened(expanded, group), ring }
        },
        CenterRow::Note { id, .. } => {
            let note = group.notes.iter().find(|note| note.id == *id)?.clone();
            rsx! {
                CardRow { note, row: row.clone(), ring, now }
            }
        }
    };
    Some(ListItem::row(
        row.key(),
        String::new(),
        rsx! {
            div { class: "sill-nc-item", "data-group": "{group.key.0}", {content} }
        },
    ))
}

/// Whether the person opened `group`.
fn opened(expanded: &Expanded, group: &AppGroup) -> ds::Shown {
    match expanded.0.contains(&group.key) {
        true => ds::Shown::Visible,
        false => ds::Shown::Hidden,
    }
}

/// A group's header row.
#[component]
fn HeadRow(group: AppGroup, expanded: ds::Shown, ring: Ring) -> Element {
    let ctx = use_context::<CenterCtx>();
    let app = group.notes.first().map(|note| note.app.clone());
    let mark = use_app_mark(&app.unwrap_or_else(nobody), IconSize::Base);
    let count = u32::try_from(group.notes.len()).unwrap_or(u32::MAX);
    let (toggle, clear) = (ctx.clone(), ctx);
    let (open, gone) = (group.key.clone(), group.key);
    rsx! {
        GroupHeader {
            icon: mark.icon,
            name: mark.name,
            count,
            expanded,
            on_toggle: move |_| toggle.act(RowAct::Toggle(open.clone()), None),
            on_clear: move |_| clear.act(RowAct::Clear(gone.clone()), None),
            common: row_data(ring, None),
        }
    }
}

/// The sender of an empty group: nobody.
fn nobody() -> sill_model::notifications::AppRef {
    sill_model::notifications::AppRef {
        name: String::new(),
        icon: sill_model::notifications::IconRef::None,
        desktop_entry: None,
    }
}

/// Whether the keyboard's ring is on the row at `at`.
fn ringed(focus: RowFocus, at: usize) -> Ring {
    match focus {
        RowFocus::Row(row) if row == at => Ring::On,
        RowFocus::Row(_) | RowFocus::None => Ring::Off,
    }
}

/// Whether a row carries the keyboard's ring.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Ring {
    On,
    Off,
}

impl Ring {
    fn attribute(self) -> &'static str {
        match self {
            Ring::On => "ring",
            Ring::Off => "none",
        }
    }
}

/// One card row.
#[component]
fn CardRow(note: Notification, row: CenterRow, ring: Ring, now: UnixMs) -> Element {
    let ctx = use_context::<CenterCtx>();
    let stack = row.stack_count();
    let app = note.app.desktop_entry.clone();
    rsx! {
        NoteCard {
            note,
            count: stack,
            place: CardPlace::Center,
            now,
            common: row_data(ring, Some(stack)),
            on_press: move |card: CardPress| {
                if let Some(act) = press(&row, card) {
                    ctx.act(act, app.clone());
                }
            },
        }
    }
}

/// The data the list's owner writes on a row's root: the keyboard's ring, and how many stand
/// behind a card.
fn row_data(ring: Ring, stack: Option<usize>) -> Common {
    let focus = DataName::parse("focus").map(|name| DataAttr::new(name, ring.attribute()));
    let stacked = stack.and_then(|count| {
        DataName::parse("stack")
            .ok()
            .map(|name| DataAttr::new(name, count.to_string()))
    });
    Common {
        data: focus.into_iter().chain(stacked).collect(),
        ..Common::default()
    }
}
