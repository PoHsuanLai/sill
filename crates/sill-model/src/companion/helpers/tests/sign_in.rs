//! The app a helper runs: how the card names it, where the sign-in stands, the buttons that
//! sign in and out, and what the answers to them do.

use companion_wire::SignInChoice;
use docket_core::helper::TerminalLogin;
use prov::AgentProgram;

use super::*;
use crate::companion::helpers::sign_in::program_name;

fn program(slug: &str) -> AgentProgram {
    AgentProgram::parse(slug).unwrap_or_else(|e| panic!("{e}"))
}

fn way(id: &str, name: &str) -> SignInChoice {
    SignInChoice {
        id: id.to_owned(),
        name: name.to_owned(),
    }
}

fn app_card(login: LoginShown, ways: Vec<SignInChoice>) -> HelperCardView {
    HelperCardView {
        kind: HelperKind::AgentApp(program("claude-code")),
        login,
        sign_in: ways,
        ..card("h-1", "Research", HelperModel::Unset)
    }
}

#[test]
fn a_helper_that_runs_an_app_names_it_in_plain_words() {
    let native = card("h-1", "Research", HelperModel::Unset);
    assert_eq!(kind_line(&native), None);
    assert_eq!(login_line(&native), None);
    let app = app_card(LoginShown::Unknown, Vec::new());
    assert_eq!(kind_line(&app).as_deref(), Some("Uses Claude Code"));
    assert_eq!(program_name("codex"), "Codex");
    let named = HelperCardView {
        app_name: Some("Claude Code CLI".to_owned()),
        ..app
    };
    assert_eq!(kind_line(&named).as_deref(), Some("Uses Claude Code CLI"));
}

#[test]
fn the_card_says_whether_the_person_is_signed_in() {
    for (login, says) in [
        (LoginShown::SignedIn, Some("Signed in")),
        (LoginShown::NeedsSignIn, Some("Waiting for you to sign in")),
        (LoginShown::Unknown, None),
    ] {
        assert_eq!(login_line(&app_card(login, Vec::new())), says);
    }
}

#[test]
fn one_way_is_a_sign_in_button_and_several_are_named_by_the_app() {
    let one = app_card(
        LoginShown::NeedsSignIn,
        vec![way("login", "Claude account")],
    );
    assert_eq!(
        sign_in_buttons(&one),
        [SignInButton {
            label: "Sign In".to_owned(),
            way: "login".to_owned()
        }]
    );
    let two = app_card(
        LoginShown::NeedsSignIn,
        vec![way("a", "Claude account"), way("b", "API key")],
    );
    let labels: Vec<String> = sign_in_buttons(&two).into_iter().map(|b| b.label).collect();
    assert_eq!(labels, ["Claude account", "API key"]);
    let signed_in = app_card(LoginShown::SignedIn, vec![way("a", "Claude account")]);
    assert!(sign_in_buttons(&signed_in).is_empty());
    assert!(offers_sign_out(&signed_in));
    assert!(!offers_sign_out(&one));
}

#[test]
fn a_job_waiting_to_sign_in_says_which_app() {
    let app = app_card(LoginShown::NeedsSignIn, Vec::new());
    assert_eq!(
        needs_sign_in_line(&app),
        "Sign in to Claude Code so this job can start."
    );
}

#[test]
fn signing_in_and_out_ask_the_companion_and_done_reads_the_card_again() {
    let model = opened("h-1", HelperModel::Unset);
    let (_, sent) = update(
        model.clone(),
        HelpersMsg::SignIn(id("h-1"), "login".to_owned()),
    );
    assert_eq!(
        sent,
        [HelpersCommand::SignIn {
            helper: id("h-1"),
            way: "login".to_owned()
        }]
    );
    let (_, sent) = update(model.clone(), HelpersMsg::SignOut(id("h-1")));
    assert_eq!(sent, [HelpersCommand::SignOut(id("h-1"))]);
    let (_, sent) = update(model, HelpersMsg::Replied(Asked::SignIn, HelperReply::Done));
    assert_eq!(
        sent,
        [
            HelpersCommand::List { after: 0 },
            HelpersCommand::ReadCard(id("h-1"))
        ]
    );
}

#[test]
fn a_terminal_request_opens_a_window_and_closing_it_asks_the_companion_to_check_again() {
    let request = docket_core::helper::TerminalRequest {
        helper: id("h-1"),
        program: program("claude-code"),
        login: TerminalLogin::default(),
        run: None,
        handoff: None,
    };
    let (_, sent) = update(
        opened("h-1", HelperModel::Unset),
        HelpersMsg::Replied(Asked::SignIn, HelperReply::OpenTerminal(request.clone())),
    );
    assert_eq!(sent, [HelpersCommand::OpenTerminal(request)]);
    let (_, sent) = update(
        HelpersModel::default(),
        HelpersMsg::TerminalClosed(id("h-1")),
    );
    assert_eq!(sent, [HelpersCommand::Resume(id("h-1"))]);
    let (model, sent) = update(HelpersModel::default(), HelpersMsg::NoTerminal);
    assert!(sent.is_empty());
    assert_eq!(
        model.notice.as_deref(),
        Some("A terminal window could not be opened.")
    );
    let (model, sent) = update(HelpersModel::default(), HelpersMsg::SignInUnconfined);
    assert!(sent.is_empty());
    assert_eq!(model.notice.as_deref(), Some("Signing in did not work."));
}

#[test]
fn a_card_read_back_keeps_the_app_the_helper_runs_when_it_is_saved() {
    let app = app_card(LoginShown::SignedIn, Vec::new());
    let draft = Draft::of(&app);
    let form = draft.form().unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(form.kind, app.kind);
}
