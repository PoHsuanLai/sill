//! One app's group: quire's `GroupHeader` (the app's icon and name, the count, "N more" or "Show
//! less", Clear) over its cards (quire's `NotificationCard` through [`NoteCard`], the newest
//! stacked over the rest while folded). The cards opening the group adds stand in quire's
//! `Reveal`, so they rise under the head as they mount (FINDINGS F1012). The keyboard's ring is
//! drawn round the header or the card it is on.

use dioxus::prelude::*;
use ds::detail::{FirstShow, Reveal};
use ds::{AppMark, GroupHeader, IconSize};
use sill_services::{AppGroup, AppRef, IconRef, UnixMs};

use super::drive::CenterCtx;
use super::groups::GroupView;
use super::keys::RowFocus;
use super::rows::{CenterRow, RowAct, press, rows_of};
use crate::surfaces::notification_card::{CardPlace, CardPress, NoteCard, use_app_mark};

/// A group, its rows numbered from `first` among the column's.
#[component]
pub(super) fn GroupBlock(
    group: Option<AppGroup>,
    view: GroupView,
    first: usize,
    focus: RowFocus,
    now: UnixMs,
) -> Element {
    let ctx = use_context::<CenterCtx>();
    let app = group
        .as_ref()
        .and_then(|group| group.notes.first())
        .map_or_else(nobody, |note| note.app.clone());
    let mark = use_app_mark(&app, IconSize::Base);
    let Some(group) = group else {
        return rsx! {};
    };
    let expanded = match ctx.expanded.read().0.contains(&group.key) {
        true => ds::Expanded::Open,
        false => ds::Expanded::Closed,
    };
    let drawn = |(n, row): (usize, CenterRow)| match row {
        CenterRow::Head { .. } => head_row(
            &ctx,
            &group,
            mark.clone(),
            expanded,
            ringed(focus, first + n),
        ),
        CenterRow::Note { .. } => card_row(&ctx, &group, row, ringed(focus, first + n), now),
    };
    let (kept, opened) = split_opened(rows_of(std::slice::from_ref(&view)));
    let kept: Vec<Element> = kept.into_iter().enumerate().map(drawn).collect();
    let at = kept.len();
    let opened: Vec<Element> = opened
        .into_iter()
        .enumerate()
        .map(|(n, row)| drawn((at + n, row)))
        .collect();
    rsx! {
        div { class: "sill-nc-group",
            {kept.into_iter()}
            if !opened.is_empty() {
                // design/26 5.4 "Expand (group in the center)": the cards an opened group adds
                // rise downward from the head, `--stagger` apart, as they mount (G33).
                Reveal { first: FirstShow::Animate,
                    {opened.into_iter()}
                }
            }
        }
    }
}

/// A group's rows split into the ones it shows folded (its header and newest card) and the ones
/// opening it adds.
fn split_opened(rows: Vec<CenterRow>) -> (Vec<CenterRow>, Vec<CenterRow>) {
    let mut rows = rows;
    let opened = rows.split_off(rows.len().min(FOLDED_ROWS));
    (rows, opened)
}

/// A folded group's rows: the header and the newest card.
const FOLDED_ROWS: usize = 2;

/// The group's header row.
fn head_row(
    ctx: &CenterCtx,
    group: &AppGroup,
    mark: AppMark,
    expanded: ds::Expanded,
    ring: Ring,
) -> Element {
    let count = u32::try_from(group.notes.len()).unwrap_or(u32::MAX);
    let (toggle, clear) = (ctx.clone(), ctx.clone());
    let (open, gone) = (group.key.clone(), group.key.clone());
    rsx! {
        div {
            key: "head-{group.key.0}",
            class: "sill-nc-group-head",
            "data-focus": ring.attribute(),
            GroupHeader {
                icon: mark.icon,
                name: mark.name,
                count,
                expanded,
                on_toggle: move |_| toggle.act(RowAct::Toggle(open.clone()), None),
                on_clear: move |_| clear.act(RowAct::Clear(gone.clone()), None),
            }
        }
    }
}

/// The sender of an empty group: nobody.
fn nobody() -> AppRef {
    AppRef {
        name: String::new(),
        icon: IconRef::None,
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
fn card_row(ctx: &CenterCtx, group: &AppGroup, row: CenterRow, ring: Ring, now: UnixMs) -> Element {
    let CenterRow::Note { id, stack, .. } = row else {
        return rsx! {};
    };
    let Some(note) = group.notes.iter().find(|note| note.id == id).cloned() else {
        return rsx! {};
    };
    let app = note.app.desktop_entry.clone();
    let on_press = ctx.clone();
    rsx! {
        div {
            key: "{id.0}",
            class: "sill-nc-row",
            "data-focus": ring.attribute(),
            "data-stack": stack.count(),
            NoteCard {
                note,
                count: stack.count(),
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
