use crate::accounts::AccountStanding;
use ds_core::word::Word;

/// The words accountd's `State` property carries (`porter-dbus`'s `state_slug`).
const CASES: &[(&str, Option<AccountStanding>, bool)] = &[
    ("ok", Some(AccountStanding::Ok), false),
    ("needs_reauth", Some(AccountStanding::NeedsReauth), true),
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
