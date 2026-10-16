//! The reducer's limits on live banners and on how long an app's critical banner stands.

use super::{
    APP_CRITICAL_HOLD_MS, AppRef, BANNERS_PER_APP_MAX, Body, CloseReason, Expiry, Hints, IconRef,
    LIVE_BANNERS_MAX, NotesState, Notification, NotificationId, NotificationsEffect,
    NotificationsEvent, NotificationsPolicy, NotifyUrgency, Quiet, Transience, UnixMs, step,
};

fn note(id: u32, app: &str, entry: Option<&str>, hints: Hints) -> Notification {
    Notification {
        id: NotificationId(id),
        arrived: UnixMs(1_000),
        app: AppRef {
            name: app.to_owned(),
            icon: IconRef::None,
            desktop_entry: entry.map(str::to_owned),
        },
        summary: format!("n{id}"),
        body: Body::default(),
        actions: Vec::new(),
        hints,
        expiry: Expiry::Default,
    }
}

fn transient() -> Hints {
    Hints {
        transience: Transience::Transient,
        ..Hints::default()
    }
}

fn critical() -> Hints {
    Hints {
        urgency: NotifyUrgency::Critical,
        ..Hints::default()
    }
}

fn post(
    state: NotesState,
    note: Notification,
    policy: &NotificationsPolicy,
) -> (NotesState, Vec<NotificationsEffect>) {
    step(state, NotificationsEvent::Posted(Box::new(note)), policy)
}

fn banner_ids(state: &NotesState) -> Vec<u32> {
    state.live.banners.iter().map(|b| b.note.id.0).collect()
}

#[test]
fn one_apps_flood_keeps_its_newest_banners_and_tells_it_about_the_rest() {
    let policy = NotificationsPolicy::default();
    let over = u32::try_from(BANNERS_PER_APP_MAX).unwrap_or(0) + 2;
    let mut state = NotesState::default();
    let mut told = Vec::new();
    for id in 1..=over {
        let (next, effects) = post(state, note(id, "chatty", None, transient()), &policy);
        state = next;
        told.extend(effects);
    }
    let (other, _) = post(state, note(over + 1, "quiet", None, transient()), &policy);
    let kept: Vec<u32> = (3..=over).rev().collect();
    let want: Vec<u32> = std::iter::once(over + 1).chain(kept).collect();
    assert_eq!(banner_ids(&other), want, "the oldest two of the flood left");
    for id in [1, 2] {
        let closed = NotificationsEffect::Closed {
            id: NotificationId(id),
            reason: CloseReason::Expired,
        };
        assert!(told.contains(&closed), "{id} told its sender: {told:?}");
    }
}

#[test]
fn many_apps_cannot_push_the_live_banners_past_the_total_or_push_out_the_shells_notice() {
    let policy = NotificationsPolicy::default();
    let shell = note(1, "Keycap", Some("sill-keycap"), Hints::default());
    let (mut state, _) = post(NotesState::default(), shell, &policy);
    let total = u32::try_from(LIVE_BANNERS_MAX).unwrap_or(0) + 20;
    for id in 2..=total {
        let app = format!("app-{id}");
        state = post(state, note(id, &app, None, Hints::default()), &policy).0;
    }
    assert_eq!(state.live.banners.len(), LIVE_BANNERS_MAX);
    assert!(
        state.live.banner(NotificationId(1)).is_some(),
        "the shell's notice outlived the apps'"
    );
    assert_eq!(
        banner_ids(&state).first(),
        Some(&total),
        "the newest is on top"
    );
}

#[test]
fn an_apps_critical_notice_gives_way_to_do_not_disturb_and_leaves_after_its_hold() {
    let dnd = NotificationsPolicy {
        quiet: Quiet::DoNotDisturb,
        ..NotificationsPolicy::default()
    };
    let (held, _) = post(
        NotesState::default(),
        note(1, "app", None, critical()),
        &dnd,
    );
    assert!(held.live.banners.is_empty(), "held back by Do Not Disturb");

    let policy = NotificationsPolicy::default();
    let shell = note(2, "Keycap", Some("sill-keycap"), critical());
    let (state, _) = post(
        NotesState::default(),
        note(1, "app", None, critical()),
        &policy,
    );
    let (state, _) = post(state, shell, &policy);
    assert_eq!(banner_ids(&state), vec![2, 1], "both persist at first");

    let held_for = APP_CRITICAL_HOLD_MS;
    let almost = UnixMs(1_000 + held_for - 1);
    let (state, _) = step(state, NotificationsEvent::Swept(almost), &policy);
    assert_eq!(banner_ids(&state), vec![2, 1], "not yet");
    let due = UnixMs(1_000 + held_for);
    let (state, _) = step(state, NotificationsEvent::Swept(due), &policy);
    assert_eq!(
        banner_ids(&state),
        vec![2],
        "the app's left, the shell's stays"
    );
}
