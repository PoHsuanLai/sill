use crate::accounts::{AccountStanding, SignInAsk};
use ds_core::word::Word;

/// The words accountd's `State` property carries (`porter-dbus`'s `state_slug`).
const CASES: &[(&str, Option<AccountStanding>, bool)] = &[
    ("ok", Some(AccountStanding::Ok), false),
    ("needs_reauth", Some(AccountStanding::NeedsReauth), true),
    ("needs_login", Some(AccountStanding::NeedsLogin), true),
    ("offline", Some(AccountStanding::Offline), false),
    ("limited", Some(AccountStanding::Limited), false),
    ("revoked", None, false),
    ("", None, false),
];

#[test]
fn accountd_words_parse_and_only_a_refused_sign_in_needs_the_person() {
    for (word, standing, needs) in CASES {
        assert_eq!(AccountStanding::parse(word), *standing, "{word:?}");
        assert_eq!(
            standing.is_some_and(AccountStanding::needs_sign_in),
            *needs,
            "{word:?}"
        );
    }
}

#[test]
fn each_sign_in_standing_asks_for_its_own_sign_in() {
    assert_eq!(AccountStanding::NeedsReauth.ask(), Some(SignInAsk::Again));
    assert_eq!(AccountStanding::NeedsLogin.ask(), Some(SignInAsk::Login));
    for quiet in [
        AccountStanding::Ok,
        AccountStanding::Offline,
        AccountStanding::Limited,
    ] {
        assert_eq!(quiet.ask(), None, "{quiet:?}");
    }
}
