//! Table tests for `reduce.rs` (its module doc is the rule list).

use super::{
    Clear, CloseReason, NotesState, NotificationsEffect, NotificationsEvent, assign_id, step,
};
use crate::notifications::content::{
    Action, AppRef, Expiry, Hints, IconRef, Notification, NotifyUrgency, Residency, SoundHint,
    Transience,
};
use crate::notifications::history_file::{STORED_VERSION, StoredHistory};
use crate::notifications::ids::{ActionKey, Activation, AppKey, NotificationId, UnixMs};
use crate::notifications::markup::Body;
use crate::notifications::policy::{HistoryCap, NotificationsPolicy, Quiet};
use crate::notifications::state::{AppGroup, History, NameHolder, ServerMode, ServerStatus};

use NotificationsEffect::{Closed, SaveHistory, Sound};
use NotificationsEvent::{
    BannerTimedOut, ClosedByApp, Dismissed, DismissedAll, HistoryCleared, Invoked, Loaded,
};

fn posted(note: Notification) -> NotificationsEvent {
    NotificationsEvent::Posted(Box::new(note))
}

fn note(id: u32, app: &str) -> Notification {
    Notification {
        id: NotificationId(id),
        arrived: UnixMs(i64::from(id) * 1000),
        app: AppRef {
            name: app.to_owned(),
            icon: IconRef::None,
            desktop_entry: None,
        },
        summary: format!("n{id}"),
        body: Body::default(),
        actions: vec![
            Action {
                key: ActionKey::default_action(),
                label: "Open".to_owned(),
            },
            Action {
                key: ActionKey("reply".to_owned()),
                label: "Reply".to_owned(),
            },
        ],
        hints: Hints::default(),
        expiry: Expiry::Default,
    }
}

fn with_hints(mut n: Notification, hints: Hints) -> Notification {
    n.hints = hints;
    n
}

fn run(
    state: NotesState,
    events: Vec<NotificationsEvent>,
    policy: &NotificationsPolicy,
) -> (NotesState, Vec<NotificationsEffect>) {
    events
        .into_iter()
        .fold((state, Vec::new()), |(s, _), e| step(s, e, policy))
}

/// The live banners' ids and each group's key and ids, in order.
fn shape(state: &NotesState) -> (Vec<u32>, Vec<(String, Vec<u32>)>) {
    let banners = state.live.banners.iter().map(|b| b.note.id.0).collect();
    let groups = state
        .history
        .groups
        .iter()
        .map(|g| (g.key.0.clone(), g.notes.iter().map(|n| n.id.0).collect()))
        .collect();
    (banners, groups)
}

fn groups(list: &[(&str, &[u32])]) -> Vec<(String, Vec<u32>)> {
    list.iter()
        .map(|(key, ids)| ((*key).to_owned(), ids.to_vec()))
        .collect()
}

/// Every wanted effect is there, and nothing else.
fn same_effects(got: &[NotificationsEffect], want: &[NotificationsEffect], case: &str) {
    assert_eq!(got.len(), want.len(), "{case}: {got:?}");
    for effect in want {
        assert!(
            got.contains(effect),
            "{case}: {effect:?} missing from {got:?}"
        );
    }
}

fn closed(id: u32, reason: CloseReason) -> NotificationsEffect {
    Closed {
        id: NotificationId(id),
        reason,
    }
}

/// Posted 1 (a), 2 (b), 3 (a) under the default policy.
fn three() -> NotesState {
    run(
        NotesState::default(),
        vec![
            posted(note(1, "a")),
            posted(note(2, "b")),
            posted(note(3, "a")),
        ],
        &NotificationsPolicy::default(),
    )
    .0
}

#[test]
fn ids_count_up_replace_when_held_and_skip_zero() {
    let held = three();
    let at_max = NotesState {
        last_id: NotificationId(u32::MAX),
        ..NotesState::default()
    };
    let cases = [
        ("first", NotesState::default(), None, 1),
        ("next", held.clone(), None, 4),
        (
            "replaces a held one",
            held.clone(),
            Some(NotificationId(2)),
            2,
        ),
        ("replaces one gone", held, Some(NotificationId(9)), 4),
        ("wraps past zero", at_max, None, 1),
    ];
    for (case, state, replaces, want) in cases {
        assert_eq!(assign_id(&state, replaces), NotificationId(want), "{case}");
    }
}

#[test]
fn a_post_shows_a_banner_enters_its_group_and_sounds() {
    let policy = NotificationsPolicy::default();
    let (state, effects) = run(NotesState::default(), vec![posted(note(1, "a"))], &policy);
    assert_eq!(shape(&state), (vec![1], groups(&[("a", &[1])])));
    assert_eq!(state.last_id, NotificationId(1));
    same_effects(&effects, &[SaveHistory, Sound(SoundHint::Default)], "first");
    assert_eq!(
        shape(&three()),
        (vec![3, 2, 1], groups(&[("a", &[3, 1]), ("b", &[2])])),
        "groups by app, newest first"
    );
}

#[test]
fn do_not_disturb_keeps_banners_and_sounds_back_except_critical() {
    let dnd = NotificationsPolicy {
        quiet: Quiet::DoNotDisturb,
        ..NotificationsPolicy::default()
    };
    let (state, effects) = run(NotesState::default(), vec![posted(note(1, "a"))], &dnd);
    assert_eq!(shape(&state), (vec![], groups(&[("a", &[1])])));
    same_effects(&effects, &[SaveHistory], "dnd");
    let critical = with_hints(
        note(2, "a"),
        Hints {
            urgency: NotifyUrgency::Critical,
            ..Hints::default()
        },
    );
    let (state, effects) = run(state, vec![posted(critical)], &dnd);
    assert_eq!(shape(&state), (vec![2], groups(&[("a", &[2, 1])])));
    same_effects(
        &effects,
        &[SaveHistory, Sound(SoundHint::Default)],
        "critical",
    );
}

#[test]
fn a_timeout_leaves_the_history_and_a_transient_one_expires() {
    let policy = NotificationsPolicy::default();
    let (state, effects) = run(three(), vec![BannerTimedOut(NotificationId(3))], &policy);
    assert_eq!(
        shape(&state),
        (vec![2, 1], groups(&[("a", &[3, 1]), ("b", &[2])]))
    );
    same_effects(&effects, &[], "kept");

    let transient = with_hints(
        note(1, "a"),
        Hints {
            transience: Transience::Transient,
            ..Hints::default()
        },
    );
    let (state, effects) = run(NotesState::default(), vec![posted(transient)], &policy);
    assert_eq!(shape(&state), (vec![1], vec![]));
    same_effects(&effects, &[Sound(SoundHint::Default)], "transient posted");
    let (state, effects) = run(state, vec![BannerTimedOut(NotificationId(1))], &policy);
    assert_eq!(shape(&state), (vec![], vec![]));
    same_effects(
        &effects,
        &[closed(1, CloseReason::Expired)],
        "transient expired",
    );

    let alert = with_hints(
        note(1, "a"),
        Hints {
            residency: Residency::Resident,
            ..Hints::default()
        },
    );
    let (posted, _) = run(NotesState::default(), vec![posted(alert)], &policy);
    let (state, effects) = run(
        posted.clone(),
        vec![BannerTimedOut(NotificationId(1))],
        &policy,
    );
    assert_eq!(state, posted, "a persisting banner does not time out");
    same_effects(&effects, &[], "persists");
}

#[test]
fn the_history_keeps_the_newest_up_to_its_cap() {
    let policy = NotificationsPolicy {
        history_cap: HistoryCap(2),
        ..NotificationsPolicy::default()
    };
    let (state, _) = run(
        NotesState::default(),
        vec![
            posted(note(1, "a")),
            posted(note(2, "b")),
            posted(note(3, "a")),
        ],
        &policy,
    );
    assert_eq!(
        shape(&state),
        (vec![3, 2, 1], groups(&[("a", &[3]), ("b", &[2])]))
    );
}

#[test]
fn a_replacement_moves_to_the_front_without_a_signal() {
    let policy = NotificationsPolicy::default();
    let mut again = note(1, "a");
    again.summary = "edited".to_owned();
    let (state, effects) = run(three(), vec![posted(again)], &policy);
    assert_eq!(
        shape(&state),
        (vec![1, 3, 2], groups(&[("a", &[1, 3]), ("b", &[2])]))
    );
    assert_eq!(
        state
            .history
            .find(NotificationId(1))
            .map(|n| n.summary.as_str()),
        Some("edited")
    );
    assert_eq!(state.last_id, NotificationId(3));
    same_effects(
        &effects,
        &[SaveHistory, Sound(SoundHint::Default)],
        "replaced",
    );
}

#[test]
fn closing_and_dismissing_remove_everywhere_with_their_reason() {
    let policy = NotificationsPolicy::default();
    let cases = [
        (
            "closed by the app",
            ClosedByApp(NotificationId(2)),
            (vec![3, 1], groups(&[("a", &[3, 1])])),
            vec![closed(2, CloseReason::ClosedByCall), SaveHistory],
        ),
        (
            "dismissed",
            Dismissed(NotificationId(3)),
            (vec![2, 1], groups(&[("a", &[1]), ("b", &[2])])),
            vec![closed(3, CloseReason::Dismissed), SaveHistory],
        ),
        (
            "not held",
            ClosedByApp(NotificationId(9)),
            (vec![3, 2, 1], groups(&[("a", &[3, 1]), ("b", &[2])])),
            vec![],
        ),
    ];
    for (case, event, want, effects) in cases {
        let (state, got) = run(three(), vec![event], &policy);
        assert_eq!(shape(&state), want, "{case}");
        same_effects(&got, &effects, case);
    }
}

#[test]
fn dismiss_all_dismisses_the_banners_in_order_and_keeps_the_rest() {
    let policy = NotificationsPolicy::default();
    let (state, _) = run(three(), vec![BannerTimedOut(NotificationId(1))], &policy);
    let (state, effects) = run(state, vec![DismissedAll], &policy);
    assert_eq!(shape(&state), (vec![], groups(&[("a", &[1])])));
    let closes: Vec<&NotificationsEffect> = effects
        .iter()
        .filter(|e| matches!(e, Closed { .. }))
        .collect();
    assert_eq!(
        closes,
        vec![
            &closed(3, CloseReason::Dismissed),
            &closed(2, CloseReason::Dismissed)
        ]
    );
    assert!(effects.contains(&SaveHistory));
}

#[test]
fn an_action_is_invoked_then_closes_unless_resident() {
    let policy = NotificationsPolicy::default();
    let reply = |id| Invoked {
        id: NotificationId(id),
        action: ActionKey("reply".to_owned()),
        token: Activation::NoToken,
    };
    let invoked = |id| NotificationsEffect::Invoked {
        id: NotificationId(id),
        action: ActionKey("reply".to_owned()),
        token: Activation::NoToken,
    };
    let (state, effects) = run(three(), vec![reply(2)], &policy);
    assert_eq!(shape(&state), (vec![3, 1], groups(&[("a", &[3, 1])])));
    same_effects(
        &effects,
        &[invoked(2), closed(2, CloseReason::Dismissed), SaveHistory],
        "closes",
    );
    let at = |e: &NotificationsEffect| effects.iter().position(|x| x == e);
    assert!(
        at(&invoked(2)) < at(&closed(2, CloseReason::Dismissed)),
        "invoked first"
    );

    let resident = with_hints(
        note(1, "a"),
        Hints {
            residency: Residency::Resident,
            ..Hints::default()
        },
    );
    let (posted, _) = run(NotesState::default(), vec![posted(resident)], &policy);
    let (state, effects) = run(posted.clone(), vec![reply(1)], &policy);
    assert_eq!(state, posted, "a resident one stays");
    same_effects(&effects, &[invoked(1)], "resident");

    let unknown = Invoked {
        id: NotificationId(2),
        action: ActionKey("delete".to_owned()),
        token: Activation::NoToken,
    };
    let (state, effects) = run(three(), vec![unknown], &policy);
    assert_eq!(state, three(), "an action it does not offer");
    same_effects(&effects, &[], "unknown action");
}

#[test]
fn clearing_a_group_or_everything_dismisses_what_it_held() {
    let policy = NotificationsPolicy::default();
    let (state, effects) = run(
        three(),
        vec![HistoryCleared(Clear::App(AppKey("a".to_owned())))],
        &policy,
    );
    assert_eq!(shape(&state), (vec![2], groups(&[("b", &[2])])));
    same_effects(
        &effects,
        &[
            closed(3, CloseReason::Dismissed),
            closed(1, CloseReason::Dismissed),
            SaveHistory,
        ],
        "app",
    );
    let (state, effects) = run(three(), vec![HistoryCleared(Clear::All)], &policy);
    assert_eq!(shape(&state), (vec![], vec![]));
    assert_eq!(
        effects
            .iter()
            .filter(|e| matches!(e, Closed { .. }))
            .count(),
        3
    );
}

#[test]
fn a_loaded_history_replaces_capped_and_keeps_ids_ahead() {
    let policy = NotificationsPolicy {
        history_cap: HistoryCap(1),
        ..NotificationsPolicy::default()
    };
    let stored = StoredHistory {
        version: STORED_VERSION,
        last_id: NotificationId(40),
        history: History {
            groups: vec![AppGroup {
                key: AppKey("a".to_owned()),
                notes: vec![note(40, "a"), note(39, "a")],
            }],
        },
    };
    let (state, effects) = run(NotesState::default(), vec![Loaded(stored)], &policy);
    assert_eq!(shape(&state), (vec![], groups(&[("a", &[40])])));
    assert_eq!(state.last_id, NotificationId(40));
    same_effects(&effects, &[], "loaded");
}

#[test]
fn the_server_status_is_published() {
    let status = ServerStatus {
        mode: ServerMode::Own,
        holder: NameHolder::Sill,
    };
    let (state, effects) = run(
        NotesState::default(),
        vec![NotificationsEvent::Server(status.clone())],
        &NotificationsPolicy::default(),
    );
    assert_eq!(state.live.server, status);
    same_effects(&effects, &[], "server");
}

#[test]
fn a_swipe_keeps_the_notification_in_the_center_unless_set_to_dismiss() {
    use crate::notifications::policy::Swipe;
    use NotificationsEvent::SwipedAway;
    let keep = NotificationsPolicy::default();
    let dismiss = NotificationsPolicy {
        swipe: Swipe::Dismiss,
        ..NotificationsPolicy::default()
    };
    let (timed_out, _) = run(three(), vec![BannerTimedOut(NotificationId(1))], &keep);
    let cases = [
        (
            "kept by default",
            three(),
            SwipedAway(NotificationId(3)),
            &keep,
            (vec![2, 1], groups(&[("a", &[3, 1]), ("b", &[2])])),
            vec![],
        ),
        (
            "dismissed when set",
            three(),
            SwipedAway(NotificationId(3)),
            &dismiss,
            (vec![2, 1], groups(&[("a", &[1]), ("b", &[2])])),
            vec![closed(3, CloseReason::Dismissed), SaveHistory],
        ),
        (
            "no banner",
            timed_out,
            SwipedAway(NotificationId(1)),
            &keep,
            (vec![3, 2], groups(&[("a", &[3, 1]), ("b", &[2])])),
            vec![],
        ),
    ];
    for (case, state, event, policy, want, effects) in cases {
        let (state, got) = run(state, vec![event], policy);
        assert_eq!(shape(&state), want, "{case}");
        same_effects(&got, &effects, case);
    }

    let transient = with_hints(
        note(1, "a"),
        Hints {
            transience: Transience::Transient,
            ..Hints::default()
        },
    );
    let (state, effects) = run(
        NotesState::default(),
        vec![posted(transient), SwipedAway(NotificationId(1))],
        &keep,
    );
    assert_eq!(shape(&state), (vec![], vec![]));
    same_effects(&effects, &[closed(1, CloseReason::Dismissed)], "transient");
}

#[test]
fn close_reasons_are_the_spec_numbers() {
    const CASES: &[(CloseReason, u32)] = &[
        (CloseReason::Expired, 1),
        (CloseReason::Dismissed, 2),
        (CloseReason::ClosedByCall, 3),
        (CloseReason::Undefined, 4),
    ];
    for &(reason, wire) in CASES {
        assert_eq!(reason.wire(), wire, "{reason:?}");
    }
}
