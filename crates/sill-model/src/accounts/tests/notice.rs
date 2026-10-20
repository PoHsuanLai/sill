use super::needing;
use crate::accounts::NeedsSignIn;
use crate::accounts::{AccountKey, SignInAsk, account_of, notice_text, sign_in_key};
use crate::notifications::ActionKey;

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
    assert_eq!(text.body, "Apps cannot use this assistant until you do.");
    assert_eq!(text.button, "Sign in");
}

#[test]
fn a_google_testing_app_sign_out_says_why() {
    let expired = NeedsSignIn {
        ask: SignInAsk::TestingExpired,
        ..needing("/a/3", "Work mail")
    };
    let text = notice_text(&expired);
    assert_eq!(text.summary, "Work mail needs you to sign in again");
    assert_eq!(
        text.body,
        "Google signs this app out every 7 days until it is verified."
    );
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
fn an_agent_with_no_launcher_says_which_app_to_open() {
    assert_eq!(
        crate::accounts::no_launcher_text("Claude Code"),
        "Open the app that runs Claude Code to sign in"
    );
}
