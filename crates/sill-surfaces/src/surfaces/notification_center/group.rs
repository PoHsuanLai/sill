//! One app's group: its header (quire `SectionHeader{Group}`: the app's name, the count, Clear;
//! FINDINGS Q125: no leading app icon until quire's header has one), its cards (lane b's
//! `NotificationCard { place: Center }`, the newest stacked over the rest while collapsed), and
//! its fold line ("N more", "Show less", `Button{Quiet}`). A row under the pointer or the
//! keyboard's ring is drawn expanded, as a hovered banner is.

use dioxus::prelude::*;
use ds::{Button, ButtonVariant, HeaderKind, SectionHeader};
use sill_launcher::AppId;
use sill_services::{AppGroup, NotificationId, UnixMs};

use super::drive::CenterCtx;
use super::groups::GroupView;
use super::keys::RowFocus;
use super::labels::{fold_label, group_title};
use super::rows::{CenterRow, RowAct, press, rows_of};
use crate::hooks::use_apps;
use crate::surfaces::notification_card::{CardPlace, CardPress, Expansion, NotificationCard};

/// A group, its rows numbered from `first` among the column's.
#[component]
pub(super) fn GroupBlock(
    group: Option<AppGroup>,
    view: GroupView,
    first: usize,
    focus: RowFocus,
    hovered: Signal<Option<NotificationId>>,
    now: UnixMs,
) -> Element {
    let ctx = use_context::<CenterCtx>();
    let apps = use_apps();
    let Some(group) = group else {
        return rsx! {};
    };
    let entry = group
        .notes
        .first()
        .and_then(|note| note.app.desktop_entry.clone())
        .and_then(|id| apps.read().get(&AppId(id)).map(|app| app.name.clone()));
    let title = group_title(&group, entry.as_deref());
    let count = group.notes.len();
    let clear = ctx.clone();
    let key = group.key.clone();
    let rows = rows_of(std::slice::from_ref(&view));
    rsx! {
        div { class: "sill-nc-group",
            SectionHeader {
                kind: HeaderKind::Group,
                text: title,
                value: (count > 1).then(|| count.to_string()),
                action: Some((
                    "Clear".to_owned(),
                    EventHandler::new(move |()| clear.act(RowAct::Clear(key.clone()), None)),
                )),
            }
            for (n, row) in rows.into_iter().enumerate() {
                {row_of(&ctx, &group, row, ringed(focus, first + n), hovered, now)}
            }
        }
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

/// One row: a card, or the fold line.
fn row_of(
    ctx: &CenterCtx,
    group: &AppGroup,
    row: CenterRow,
    ring: Ring,
    mut hovered: Signal<Option<NotificationId>>,
    now: UnixMs,
) -> Element {
    let (on_press, on_fold) = (ctx.clone(), ctx.clone());
    match row.clone() {
        CenterRow::Note { id, stack, .. } => {
            let Some(note) = group.notes.iter().find(|note| note.id == id).cloned() else {
                return rsx! {};
            };
            let app = note.app.desktop_entry.clone();
            let expansion = match (ring, *hovered.read() == Some(id)) {
                (Ring::On, _) | (Ring::Off, true) => Expansion::Expanded,
                (Ring::Off, false) => Expansion::Collapsed,
            };
            rsx! {
                div {
                    key: "{id.0}",
                    class: "sill-nc-row",
                    "data-focus": ring.attribute(),
                    "data-stack": stack.count(),
                    onpointerenter: move |_| hovered.set(Some(id)),
                    onpointerleave: move |_| {
                        if *hovered.peek() == Some(id) {
                            hovered.set(None);
                        }
                    },
                    NotificationCard {
                        note,
                        count: stack.count(),
                        expansion,
                        place: CardPlace::Center,
                        now,
                        on_press: move |card: CardPress| {
                            if let Some(act) = press(&row, card) {
                                on_press.act(act, app.clone());
                            }
                        },
                    }
                }
            }
        }
        CenterRow::Fold { group: key, fold } => rsx! {
            div {
                key: "fold-{key.0}",
                class: "sill-nc-fold",
                "data-focus": ring.attribute(),
                Button {
                    variant: ButtonVariant::Quiet,
                    label: fold_label(fold),
                    onclick: move |_| on_fold.act(RowAct::Toggle(key.clone()), None),
                }
            }
        },
    }
}
