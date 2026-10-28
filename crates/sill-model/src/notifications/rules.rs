//! The reducer's rules, one function each (`reduce.rs`'s module doc is the list): small pure
//! steps over [`NotesState`] that return the state and the effects they imply.

use super::content::{Notification, NotifyUrgency, Residency, Transience};
use super::ids::{ActionKey, Activation, NotificationId, UnixMs};
use super::limits::{APP_CRITICAL_HOLD_MS, BANNERS_PER_APP_MAX, LIVE_BANNERS_MAX};
use super::origin::Origin;
use super::policy::{Chime, HistoryCap, NotificationsPolicy, Presented, chime, presented};
use super::stored::StoredHistory;
use super::{
    AppGroup, BannerLife, Clear, CloseReason, History, LiveBanner, NotesState, NotificationsEffect,
};

type Step = (NotesState, Vec<NotificationsEffect>);

/// Whether the service holds `id`: a live banner or a kept notification.
pub fn holds(state: &NotesState, id: NotificationId) -> bool {
    find(state, id).is_some()
}

/// The notification `id`, from its banner or the history.
pub(super) fn find(state: &NotesState, id: NotificationId) -> Option<&Notification> {
    state
        .live
        .banner(id)
        .map(|banner| &banner.note)
        .or_else(|| state.history.find(id))
}

/// A posting, or a replacement of a held one (removed silently first).
pub(super) fn post(state: NotesState, note: Notification, policy: &NotificationsPolicy) -> Step {
    let id = note.id;
    let (mut state, replaced) = remove_everywhere(state, id);
    if replaced.is_none() {
        state.last_id = id;
    }
    let mut effects = Vec::new();
    let had_kept = replaced == Some(true);
    let kept = match note.hints.transience {
        Transience::Kept => {
            state.history = cap(keep(state.history, note.clone()), policy.history_cap);
            true
        }
        Transience::Transient => false,
    };
    if had_kept || kept {
        effects.push(NotificationsEffect::SaveHistory);
    }
    if let Presented::Banner(life) = presented(&note, policy) {
        state.live.banners.insert(
            0,
            LiveBanner {
                note: note.clone(),
                life,
            },
        );
        effects.extend(enforce_banner_limits(&mut state, &note));
    }
    if let Chime::Play(sound) = chime(&note, policy) {
        effects.push(NotificationsEffect::Sound(sound));
    }
    (state, effects)
}

/// The oldest banners past the limits leave, so one app's flood neither buries the others nor
/// grows what the shell holds: at most [`BANNERS_PER_APP_MAX`] for the app of `newest` (the
/// shell's own notices are not counted against it), then at most [`LIVE_BANNERS_MAX`] in all,
/// an app's banner before a shell notice. A banner that leaves and is not in the history (a
/// transient one) closes as expired.
fn enforce_banner_limits(
    state: &mut NotesState,
    newest: &Notification,
) -> Vec<NotificationsEffect> {
    let key = newest.app.key();
    let oldest_first = || state.live.banners.iter().rev();
    let mine: Vec<NotificationId> = oldest_first()
        .filter(|banner| {
            banner.note.app.key() == key && Origin::of(&banner.note.app) == Origin::App
        })
        .map(|banner| banner.note.id)
        .collect();
    let over_app = mine.len().saturating_sub(BANNERS_PER_APP_MAX);
    let mut gone: Vec<NotificationId> = mine.into_iter().take(over_app).collect();
    let over_all = state
        .live
        .banners
        .len()
        .saturating_sub(gone.len())
        .saturating_sub(LIVE_BANNERS_MAX);
    let (apps, shell): (Vec<_>, Vec<_>) = oldest_first()
        .filter(|banner| !gone.contains(&banner.note.id))
        .map(|banner| (banner.note.id, Origin::of(&banner.note.app)))
        .partition(|(_, origin)| *origin == Origin::App);
    let evicted: Vec<NotificationId> = apps
        .into_iter()
        .chain(shell)
        .take(over_all)
        .map(|(id, _)| id)
        .collect();
    gone.extend(evicted);
    leave_banners(state, &gone)
}

/// `ids` off the banners; one the history does not hold closes as expired.
fn leave_banners(state: &mut NotesState, ids: &[NotificationId]) -> Vec<NotificationsEffect> {
    state
        .live
        .banners
        .retain(|banner| !ids.contains(&banner.note.id));
    ids.iter()
        .filter(|id| state.history.find(**id).is_none())
        .map(|id| NotificationsEffect::Closed {
            id: *id,
            reason: CloseReason::Expired,
        })
        .collect()
}

/// An app's critical banners that have stood for [`APP_CRITICAL_HOLD_MS`] at `now` leave: an app
/// cannot keep a banner on the screen for ever. The shell's own stay until acted on.
pub(super) fn sweep(mut state: NotesState, now: UnixMs) -> Step {
    let stale: Vec<NotificationId> = state
        .live
        .banners
        .iter()
        .filter(|banner| {
            banner.note.hints.urgency == NotifyUrgency::Critical
                && Origin::of(&banner.note.app) == Origin::App
                && now.since(banner.note.arrived) >= APP_CRITICAL_HOLD_MS.unsigned_abs()
        })
        .map(|banner| banner.note.id)
        .collect();
    let effects = leave_banners(&mut state, &stale);
    (state, effects)
}

/// `CloseNotification` or a dismissal: removed everywhere with `reason`.
pub(super) fn close(state: NotesState, id: NotificationId, reason: CloseReason) -> Step {
    let (state, removed) = remove_everywhere(state, id);
    let effects = match removed {
        None => Vec::new(),
        Some(true) => vec![
            NotificationsEffect::Closed { id, reason },
            NotificationsEffect::SaveHistory,
        ],
        Some(false) => vec![NotificationsEffect::Closed { id, reason }],
    };
    (state, effects)
}

/// A banner swiped away under `Swipe::KeepInCenter`: the banner leaves and the notification
/// stays in the history, still live for its actions from the notification center, so no
/// `NotificationClosed` goes out (as for a banner that times out); the app hears `Closed` only when
/// the notification truly leaves (its close button, Clear, `CloseNotification`). A transient one,
/// which the history never held, is gone for good, so it does close (reason 2).
pub(super) fn swipe_away(mut state: NotesState, id: NotificationId) -> Step {
    if state.live.banner(id).is_none() {
        return (state, Vec::new());
    }
    state.live.banners.retain(|banner| banner.note.id != id);
    let effects = match state.history.find(id) {
        Some(_) => Vec::new(),
        None => vec![NotificationsEffect::Closed {
            id,
            reason: CloseReason::Dismissed,
        }],
    };
    (state, effects)
}

/// Each of `ids` dismissed in order, with one `SaveHistory` at the end if any was kept.
pub(super) fn dismiss_each(state: NotesState, ids: Vec<NotificationId>) -> Step {
    let (state, effects) = ids
        .into_iter()
        .fold((state, Vec::new()), |(state, mut effects), id| {
            let (state, more) = close(state, id, CloseReason::Dismissed);
            effects.extend(more);
            (state, effects)
        });
    (state, one_save(effects))
}

/// The live banners' ids, in order.
pub(super) fn banner_ids(state: &NotesState) -> Vec<NotificationId> {
    state.live.banners.iter().map(|b| b.note.id).collect()
}

/// The ids `clear` covers, group by group, newest first.
pub(super) fn cleared_ids(state: &NotesState, clear: &Clear) -> Vec<NotificationId> {
    state
        .history
        .groups
        .iter()
        .filter(|group| match clear {
            Clear::All => true,
            Clear::App(key) => group.key == *key,
        })
        .flat_map(|group| group.notes.iter().map(|note| note.id))
        .collect()
}

/// A banner's hold ran out: it leaves; a transient one (not kept) expires.
pub(super) fn time_out(mut state: NotesState, id: NotificationId) -> Step {
    let expires = state
        .live
        .banner(id)
        .is_some_and(|banner| banner.life == BannerLife::Expires);
    if !expires {
        return (state, Vec::new());
    }
    state.live.banners.retain(|banner| banner.note.id != id);
    let effects = match state.history.find(id) {
        Some(_) => Vec::new(),
        None => vec![NotificationsEffect::Closed {
            id,
            reason: CloseReason::Expired,
        }],
    };
    (state, effects)
}

/// An action chosen: invoked when offered, then closed unless resident.
pub(super) fn invoke(
    state: NotesState,
    id: NotificationId,
    action: ActionKey,
    token: Activation,
) -> Step {
    let residency = match find(&state, id) {
        Some(note) if note.actions.iter().any(|a| a.key == action) => note.hints.residency,
        _ => return (state, Vec::new()),
    };
    let invoked = NotificationsEffect::Invoked { id, action, token };
    match residency {
        Residency::Resident => (state, vec![invoked]),
        Residency::ClosesOnAction => {
            let (state, closing) = close(state, id, CloseReason::Dismissed);
            (state, std::iter::once(invoked).chain(closing).collect())
        }
    }
}

/// The history file read at start replaces the history.
pub(super) fn load(
    mut state: NotesState,
    stored: StoredHistory,
    policy: &NotificationsPolicy,
) -> Step {
    state.history = cap(stored.history, policy.history_cap);
    state.last_id = state.last_id.max(stored.last_id);
    (state, Vec::new())
}

/// `id` taken off the banners and out of the history; `None` when neither held it, else whether
/// it was in the history (`true`, maybe on a banner too) or only on a banner (`false`).
fn remove_everywhere(mut state: NotesState, id: NotificationId) -> (NotesState, Option<bool>) {
    let on_banner = state.live.banner(id).is_some();
    state.live.banners.retain(|banner| banner.note.id != id);
    let kept = state.history.find(id).is_some();
    if kept {
        state.history = without(state.history, id);
    }
    let removed = match (kept, on_banner) {
        (true, _) => Some(true),
        (false, true) => Some(false),
        (false, false) => None,
    };
    (state, removed)
}

/// `history` without `id`, and without a group left empty.
fn without(history: History, id: NotificationId) -> History {
    let groups = history
        .groups
        .into_iter()
        .map(|mut group| {
            group.notes.retain(|note| note.id != id);
            group
        })
        .filter(|group| !group.notes.is_empty())
        .collect();
    History { groups }
}

/// `note` at the front of its app's group, the group moved to the front.
fn keep(history: History, note: Notification) -> History {
    let key = note.app.key();
    let (mut mine, others): (Vec<AppGroup>, Vec<AppGroup>) =
        history.groups.into_iter().partition(|g| g.key == key);
    let mut notes = mine.pop().map(|g| g.notes).unwrap_or_default();
    notes.insert(0, note);
    let groups = std::iter::once(AppGroup { key, notes })
        .chain(others)
        .collect();
    History { groups }
}

/// `history` holding at most `cap`, the oldest arrivals dropped first.
fn cap(mut history: History, cap: HistoryCap) -> History {
    while history.len() > usize::from(cap.0) {
        let oldest = history
            .groups
            .iter()
            .flat_map(|group| group.notes.iter())
            .min_by_key(|note| (note.arrived, note.id))
            .map(|note| note.id);
        match oldest {
            Some(id) => history = without(history, id),
            None => break,
        }
    }
    history
}

/// `effects` with every `SaveHistory` folded into one at the end.
fn one_save(effects: Vec<NotificationsEffect>) -> Vec<NotificationsEffect> {
    let saves = effects.contains(&NotificationsEffect::SaveHistory);
    let mut rest: Vec<NotificationsEffect> = effects
        .into_iter()
        .filter(|e| *e != NotificationsEffect::SaveHistory)
        .collect();
    if saves {
        rest.push(NotificationsEffect::SaveHistory);
    }
    rest
}
