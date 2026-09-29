//! `use_notification_history`: every kept notification, grouped by app.

use dioxus::prelude::{ReadSignal, ReadableExt, use_memo};
use shell_host::use_watch;
use sill_services::History;

use super::use_services;

/// The history, groups and their notifications newest first: what the notification center
/// draws. A separate receiver from [`super::use_notifications`], so a banner coming and going
/// does not repaint a closed center.
pub fn use_notification_history() -> ReadSignal<History> {
    let notes = use_watch(use_services().notifications);
    ReadSignal::from(use_memo(move || notes.read().history.clone()))
}
