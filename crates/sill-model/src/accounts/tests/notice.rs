use super::needing;
use crate::accounts::{AccountKey, NOTICE_ENTRY, account_of, notice_text, shown_for, sign_in_key};
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
