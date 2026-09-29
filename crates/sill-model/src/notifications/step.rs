//! The one place the service's state changes: [`step`], pure, with the D-Bus
//! signals, sounds and history writes it implies returned as [`NotificationsEffect`]s for the
//! live loop to carry out (CONVENTIONS §6: an effect is returned, not performed).
//!
//! The rules, which `reduce_tests.rs` tables:
//!
//! - **Posted.** A new id raises `last_id`. Unless `transient`, the notification enters the front
//!   of its app's group and the group moves to the front; past `history_cap` the oldest kept
//!   notification anywhere is dropped (silently: it may still be a live banner). Then it presents
//!   per [`presented`](super::policy::presented): a banner goes to the front of `banners`. A
//!   [`chime`](super::policy::chime) that plays is a `Sound`. A history change is `SaveHistory`.
//! - **A replacement** (a posting whose id is already held, from `replaces_id`) is the old one
//!   removed without a signal, then the new one posted as above: it moves to the front and shows
//!   again.
//! - **ClosedByApp** (`CloseNotification`), **Dismissed** (the close button, a history row's
//!   clear): removed from the banners and the history, `Closed` with `ClosedByCall` or
//!   `Dismissed`. An id not held changes nothing.
//! - **SwipedAway** (a banner swiped or dragged off): under `notifications.swipe =
//!   "keep_in_center"` (the default, as macOS) the banner leaves, the notification stays in the
//!   history, and `Closed` says `Dismissed` (the spec's "dismissed by the user": the banner is
//!   gone); a transient one, which the history never held, is gone entirely. Under `"dismiss"`
//!   it is **Dismissed**. An id with no banner changes nothing.
//! - **DismissedAll** (`sill notifications dismiss-all`): every live banner dismissed, in banner
//!   order. The history keeps what is not on screen.
//! - **BannerTimedOut**: the banner leaves; the notification stays in the history. One the
//!   history does not hold (transient) is `Closed` with `Expired`. A persisting banner never
//!   times out: the event is ignored.
//! - **Invoked**: an action the notification offers is `Invoked` (the live loop emits
//!   `ActivationToken` first when it has one, then `ActionInvoked`); unless `resident`, the
//!   notification is then removed everywhere and `Closed` with `Dismissed`. An action it does
//!   not offer, or an id not held, changes nothing.
//! - **HistoryCleared**: every notification in the cleared groups is dismissed (its banner too).
//! - **Loaded**: the stored history replaces the history (capped), `last_id` becomes the larger
//!   of the two. No `SaveHistory`.
//! - **Server**: the bus name's news.

use super::ids::NotificationId;
use super::policy::{NotificationsPolicy, Swipe};
use super::{CloseReason, NotesState, NotificationsEffect, NotificationsEvent, rules};

/// The id a posting gets: the one it replaces when that one is still held (a live banner or
/// in the history), else one past `last_id`, skipping 0 when it wraps.
pub fn assign_id(state: &NotesState, replaces: Option<NotificationId>) -> NotificationId {
    match replaces {
        Some(id) if rules::holds(state, id) => id,
        _ => {
            let after = |id: NotificationId| match id.0.wrapping_add(1) {
                0 => NotificationId(1),
                next => NotificationId(next),
            };
            // Past a wrap, an id the history still holds is skipped (bounded: the history is
            // capped far below u32::MAX).
            std::iter::successors(Some(after(state.last_id)), |id| Some(after(*id)))
                .take(usize::from(u16::MAX) + 2)
                .find(|id| !rules::holds(state, *id))
                .unwrap_or_else(|| after(state.last_id))
        }
    }
}

/// `state` after `event` under `policy`, and what must happen because of it (module doc).
pub fn step(
    state: NotesState,
    event: NotificationsEvent,
    policy: &NotificationsPolicy,
) -> (NotesState, Vec<NotificationsEffect>) {
    match event {
        NotificationsEvent::Posted(note) => rules::post(state, *note, policy),
        NotificationsEvent::ClosedByApp(id) => rules::close(state, id, CloseReason::ClosedByCall),
        NotificationsEvent::Dismissed(id) => rules::close(state, id, CloseReason::Dismissed),
        NotificationsEvent::SwipedAway(id) => match policy.swipe {
            Swipe::KeepInCenter => rules::swipe_away(state, id),
            Swipe::Dismiss => rules::close(state, id, CloseReason::Dismissed),
        },
        NotificationsEvent::DismissedAll => {
            let ids = rules::banner_ids(&state);
            rules::dismiss_each(state, ids)
        }
        NotificationsEvent::BannerTimedOut(id) => rules::time_out(state, id),
        NotificationsEvent::Invoked { id, action, token } => {
            rules::invoke(state, id, action, token)
        }
        NotificationsEvent::HistoryCleared(clear) => {
            let ids = rules::cleared_ids(&state, &clear);
            rules::dismiss_each(state, ids)
        }
        NotificationsEvent::Loaded(stored) => rules::load(state, stored, policy),
        NotificationsEvent::Server(status) => {
            let mut state = state;
            state.live.server = status;
            (state, Vec::new())
        }
    }
}

#[cfg(test)]
mod tests;
