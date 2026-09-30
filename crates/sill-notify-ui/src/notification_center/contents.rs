//! Inside the column: the title with Clear All (`Button{Quiet}`), the history grouped by app
//! (`group_views`) as one quire `LeavingList` of headers and cards (each drawn by `group.rs`),
//! or the empty state; the footer when sill is not the one receiving notifications; and the
//! widgets slot at the bottom. The column holds the keyboard: every key goes through
//! [`step_keys`] over the drawn rows.
//!
//! The list's motion is quire's: the rows are simply there as the center opens, a row that
//! arrives while it is open enters with `row-in`, and every row the model drops in one update (a
//! Clear, Clear All, a dismissal, "Show less") fades and slides up, then the rows below close the
//! gap.

use std::time::{SystemTime, UNIX_EPOCH};

use dioxus::prelude::*;
use ds::{Button, ButtonVariant, LeavingItem, LeavingList};
use shell_host::OutputId;
use sill_model::notifications::{History, UnixMs};

use super::drawn::{Rows, use_drawn};
use super::group::row_item;
use super::groups::group_views;
use super::io::CenterCtx;
use super::keymap::center_key;
use super::keys::{CenterOut, RowFocus, step_keys};
use super::labels::{EMPTY, TITLE, server_line};
use super::layer::PANEL_ID;
use super::rows::{CenterRow, RowKey, activate, dismiss, rows_of};
use super::widgets::CenterWidgets;
use sill_services::{ClockService, NotificationsService};
use sill_ui_kit::hooks::{use_service, use_service_part};

/// The column's contents.
#[component]
pub(super) fn Contents(output: OutputId) -> Element {
    let ctx = use_context::<CenterCtx>();
    let history = use_service_part::<NotificationsService, _>(|notes| notes.history.clone())();
    let server = use_service_part::<NotificationsService, _>(|notes| notes.live.clone())
        .read()
        .server
        .clone();
    // The ages count in minutes: the clock's tick redraws them.
    let _minute = use_service::<ClockService>()();
    let now = wall_clock();
    let mut focus = use_signal(RowFocus::default);
    let expanded = ctx.expanded.read().clone();
    let rows = rows_of(&group_views(&history, &expanded));
    let keys = (ctx.clone(), rows.clone());
    let clear = ctx.clone();
    let home = ctx.clone();
    let items: Vec<LeavingItem<RowKey>> = rows
        .iter()
        .enumerate()
        .filter_map(|row| row_item(&history, &expanded, row, focus(), now))
        .collect();
    let listed: Vec<RowKey> = items.iter().map(|item| item.key.clone()).collect();
    let drawn = use_drawn(&listed);
    rsx! {
        div {
            id: PANEL_ID,
            class: "sill-nc-column",
            // macOS's Notification Center rubber-bands (quire design/11 §11.3.7); the launcher's
            // result list and every menu are excluded by that table and carry nothing here.
            "data-overscroll": "band",
            tabindex: "-1",
            onmounted: move |event: MountedEvent| home.home(event.data()),
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
                if drawn.rows() == Rows::None {
                    div { class: "sill-nc-empty", {EMPTY} }
                }
                LeavingList::<RowKey> {
                    label: TITLE.to_owned(),
                    items,
                    on_settled: move |key: RowKey| {
                        eprintln!("notification_center.settled={key:?}");
                        drawn.settled(&key);
                    },
                }
            }
            if let Some(line) = server_line(&server) {
                div { class: "sill-nc-foot", {line} }
            }
            div { class: "sill-nc-widgets", CenterWidgets { output } }
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

/// The wall clock the ages are counted against (the history's arrival times are wall-clock).
fn wall_clock() -> UnixMs {
    let since = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    UnixMs(i64::try_from(since.as_millis()).unwrap_or(i64::MAX))
}
