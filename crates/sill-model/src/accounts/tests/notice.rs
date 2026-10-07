use super::needing;
use crate::accounts::NeedsSignIn;
use crate::accounts::{
    AccountKey, NOTICE_ENTRY, Posted, SignInAsk, account_of, notice_text, shown_for, sign_in_key,
};
use crate::notifications::{
    Action, ActionKey, AppRef, BannerLife, Body, Expiry, Hints, IconRef, LiveBanner, NotesState,
    Notification, NotificationId, UnixMs,
};

fn note(id: u32, entry: &str, keys: &[ActionKey]) -> Notification {
    Notification {
        id: NotificationId(id),
        arrived: UnixMs(0),
        app: AppRef {
            name: String::new(),
            icon: IconRef::None,
            desktop_entry: Some(entry.to_owned()),
        },
        summary: String::new(),
        body: Body::plain(""),
        actions: keys
            .iter()
            .map(|key| Action {
                key: key.clone(),
                label: String::new(),
            })
            .collect(),
        hints: Hints::default(),
        expiry: Expiry::Default,
    }
}

fn banner(note: Notification) -> LiveBanner {
    LiveBanner {
        note,
        life: BannerLife::default(),
    }
}

#[test]
fn the_notice_names_the_account_and_the_button_says_what_it_does() {
    let text = notice_text(&needing("/a/1", "Work mail"));
    assert_eq!(text.summary, "Work mail needs you to sign in again");
    assert_eq!(text.button, "Sign in again");
}

#[test]
fn an_agent_that_must_be_signed_in_has_its_own_words() {
    let agent = NeedsSignIn {
        ask: SignInAsk::Login,
        ..needing("/a/2", "Claude Code")
    };
    let text = notice_text(&agent);
    assert_eq!(text.summary, "Sign in to Claude Code to use it again");
    assert_eq!(text.body, "Apps cannot use this agent until you do.");
    assert_eq!(text.button, "Sign in");
}

#[test]
fn an_action_key_carries_its_account_and_no_other_key_is_one() {
    let account = AccountKey::new("/org/quire/Accounts1/account/work");
    assert_eq!(account_of(&sign_in_key(&account)), Some(account));
    for other in ["default", "", "sign-in-again:", "open"] {
        assert_eq!(account_of(&ActionKey(other.to_owned())), None, "{other:?}");
    }
}

#[test]
fn the_notice_shown_is_found_by_its_account_among_banners_and_history() {
    let (a, b) = (AccountKey::new("/a"), AccountKey::new("/b"));
    let mut notes = NotesState::default();
    notes.live.banners = vec![
        banner(note(1, NOTICE_ENTRY, &[sign_in_key(&a)])),
        // Another app's action that happens to carry the same key is not ours.
        banner(note(2, "other", &[sign_in_key(&b)])),
    ];
    assert_eq!(shown_for(&notes, &a), Some(NotificationId(1)));
    assert_eq!(shown_for(&notes, &b), None);

    notes.live.banners.clear();
    notes.history.groups = vec![crate::notifications::AppGroup {
        key: crate::notifications::AppKey(NOTICE_ENTRY.to_owned()),
        notes: vec![note(7, NOTICE_ENTRY, &[sign_in_key(&b)])],
    }];
    assert_eq!(shown_for(&notes, &b), Some(NotificationId(7)));
    assert_eq!(shown_for(&notes, &a), None);
}

#[test]
fn a_press_answers_only_for_the_id_posted_for_that_account() {
    let (a, b) = (AccountKey::new("/a"), AccountKey::new("/b"));
    let mut notes = NotesState::default();
    notes.live.banners = vec![
        banner(note(1, NOTICE_ENTRY, &[sign_in_key(&a)])),
        banner(note(2, NOTICE_ENTRY, &[sign_in_key(&b)])),
    ];
    let mut posted = Posted::default();
    posted.learn(&notes, [&a, &b]);
    let id = NotificationId;
    assert_eq!(posted.answers(id(1), &sign_in_key(&a)), Some(a.clone()));
    assert_eq!(posted.answers(id(2), &sign_in_key(&b)), Some(b.clone()));
    // The right id with the wrong account, a foreign id, and a key that is not a sign-in.
    assert_eq!(posted.answers(id(1), &sign_in_key(&b)), None);
    assert_eq!(posted.answers(id(9), &sign_in_key(&a)), None);
    assert_eq!(
        posted.answers(id(1), &ActionKey("default".to_owned())),
        None
    );

    // A pressed button leaves the notes before its signal is heard; the id still answers.
    notes.live.banners.clear();
    posted.learn(&notes, [&a, &b]);
    assert_eq!(posted.answers(id(1), &sign_in_key(&a)), Some(a.clone()));
    // Closed by the server: it answers nothing, and the other account's is untouched.
    posted.closed(id(1));
    assert_eq!(posted.answers(id(1), &sign_in_key(&a)), None);
    assert_eq!(posted.answers(id(2), &sign_in_key(&b)), Some(b.clone()));
    // The account well again forgets its id.
    posted.forget(&b);
    assert_eq!(posted.answers(id(2), &sign_in_key(&b)), None);
}

#[test]
fn a_fresh_post_replaces_the_id_kept() {
    let a = AccountKey::new("/a");
    let mut notes = NotesState::default();
    notes.live.banners = vec![banner(note(1, NOTICE_ENTRY, &[sign_in_key(&a)]))];
    let mut posted = Posted::default();
    posted.learn(&notes, [&a]);
    notes.live.banners = vec![banner(note(5, NOTICE_ENTRY, &[sign_in_key(&a)]))];
    posted.learn(&notes, [&a]);
    assert_eq!(posted.answers(NotificationId(1), &sign_in_key(&a)), None);
    assert_eq!(posted.answers(NotificationId(5), &sign_in_key(&a)), Some(a));
}

#[test]
fn an_agent_with_no_launcher_says_which_app_to_open() {
    assert_eq!(
        crate::accounts::no_launcher_text("Claude Code"),
        "Open the app that runs Claude Code to sign in"
    );
}
