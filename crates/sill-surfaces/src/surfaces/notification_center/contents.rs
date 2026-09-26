//! Inside the column: the title with Clear All (`Button{Quiet}`), the history grouped by app
//! (`group_views`, each group drawn by `group.rs`), or the empty state; the footer when sill is
//! not the one receiving notifications; and the widgets slot at the bottom. The column holds the
//! keyboard: every key goes through [`step_keys`] over the drawn rows.

use std::rc::Rc;
use std::time::{SystemTime, UNIX_EPOCH};

use dioxus::prelude::*;
use ds::{Button, ButtonVariant};
use sill_services::{History, UnixMs};

use super::drive::CenterCtx;
use super::group::GroupBlock;
use super::groups::group_views;
use super::keymap::center_key;
use super::keys::{CenterOut, RowFocus, step_keys};
use super::labels::{EMPTY, TITLE, server_line};
use super::layer::PANEL_ID;
use super::rows::{CenterRow, activate, dismiss, rows_of};
use super::widgets::CenterWidgets;
use crate::hooks::{use_clock, use_notification_history, use_notifications};

/// The column's contents.
#[component]
pub(super) fn Contents() -> Element {
    let ctx = use_context::<CenterCtx>();
    let history = use_notification_history()();
    let server = use_notifications().read().server.clone();
    // The ages count in minutes: the clock's tick redraws them.
    let _minute = use_clock()();
    let now = wall_clock();
    let mut focus = use_signal(RowFocus::default);
    let views = group_views(&history, &ctx.expanded.read());
    let rows = rows_of(&views);
    let keys = (ctx.clone(), rows.clone());
    let clear = ctx.clone();
    // Each group with the index of its first row among the column's.
    let groups: Vec<_> = views
        .into_iter()
        .scan(0, |first, view| {
            let at = *first;
            *first += rows_of(std::slice::from_ref(&view)).len();
            Some((at, view))
        })
        .collect();
    rsx! {
        div {
            id: PANEL_ID,
            class: "sill-nc-column",
            // macOS's Notification Center rubber-bands (quire design/11 §11.3.7); the launcher's
            // result list and every menu are excluded by that table and carry nothing here.
            "data-overscroll": "band",
            tabindex: "-1",
            onmounted: move |event: MountedEvent| take_keyboard(event.data()),
            onkeydown: move |event: KeyboardEvent| {
                let Some(key) = center_key(&event.key()) else { return };
                event.prevent_default();
                event.stop_propagation();
                let (ctx, rows) = &keys;
                let (next, outs) = step_keys(*focus.peek(), key, rows.len());
                focus.set(next);
                for out in outs {
                    run_out(ctx, rows, &history, out);
                }
            },
            div { class: "sill-nc-head",
                span { class: "sill-nc-title", {TITLE} }
                if !history.groups.is_empty() {
                    Button {
                        variant: ButtonVariant::Quiet,
                        label: "Clear All",
                        onclick: move |_| clear.clear_all(),
                    }
                }
            }
            div { class: "sill-nc-list",
                if history.groups.is_empty() {
                    div { class: "sill-nc-empty", {EMPTY} }
                }
                for (at, view) in groups {
                    GroupBlock {
                        key: "{view.key.0}",
                        group: history.groups.iter().find(|g| g.key == view.key).cloned(),
                        view,
                        first: at,
                        focus: focus(),
                        now,
                    }
                }
            }
            if let Some(line) = server_line(&server) {
                div { class: "sill-nc-foot", {line} }
            }
            div { class: "sill-nc-widgets", CenterWidgets {} }
        }
    }
}

/// What a key's output asks: close, or the row's Return or Delete.
fn run_out(ctx: &CenterCtx, rows: &[CenterRow], history: &History, out: CenterOut) {
    let row = |at: usize| rows.get(at);
    match out {
        CenterOut::Close => ctx.close(),
        CenterOut::Activate(at) => {
            if let Some(row) = row(at) {
                ctx.act(activate(row), app_of(history, row));
            }
        }
        CenterOut::Dismiss(at) => {
            if let Some(row) = row(at) {
                ctx.act(dismiss(row), app_of(history, row));
            }
        }
    }
}

/// The desktop entry of a row's sender, for the activation token.
pub(super) fn app_of(history: &History, row: &CenterRow) -> Option<String> {
    let CenterRow::Note { id, .. } = row else {
        return None;
    };
    history
        .groups
        .iter()
        .flat_map(|group| &group.notes)
        .find(|note| note.id == *id)
        .and_then(|note| note.app.desktop_entry.clone())
}

/// The column takes the keyboard as it mounts, so the arrows and Escape reach it.
fn take_keyboard(element: Rc<MountedData>) {
    ds::focus_soon(element);
}

/// The wall clock the ages are counted against (the history's arrival times are wall-clock).
fn wall_clock() -> UnixMs {
    let since = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    UnixMs(i64::try_from(since.as_millis()).unwrap_or(i64::MAX))
}
