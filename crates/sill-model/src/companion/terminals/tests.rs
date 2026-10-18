use companion_wire::{BroughtBack, Handover, TerminalSession};
use prov::{AgentProgram, SessionId};

use super::*;

fn id(text: &str) -> SessionId {
    SessionId::parse(text).unwrap_or_else(|e| panic!("{e}"))
}

fn session(text: &str, app: Option<&str>, folder: Option<&str>) -> TerminalSession {
    TerminalSession {
        session: id(text),
        program: AgentProgram::parse("claude-code").unwrap_or_else(|e| panic!("{e}")),
        app_name: app.map(str::to_owned),
        folder: folder.map(str::to_owned),
        handover: Handover::BringBack,
    }
}

#[test]
fn a_row_says_where_the_session_is_and_which_app_runs_it() {
    let named = session("s-1", Some("Claude Code"), Some("/home/me/notes"));
    assert_eq!(session_title(&named), "In Terminal \u{2014} Claude Code");
    assert_eq!(session_subtitle(&named), "/home/me/notes");
    let unnamed = session("s-2", Some("  "), None);
    assert_eq!(
        session_title(&unnamed),
        "In Terminal \u{2014} Claude Code",
        "a blank name falls back to the program's, in words"
    );
    assert_eq!(session_subtitle(&unnamed), "");
}

#[test]
fn the_list_is_read_when_the_companion_says_it_changed_and_replaces_what_was_held() {
    let (model, sent) = update(TerminalsModel::default(), TerminalsMsg::Changed);
    assert_eq!(sent, [TerminalsCommand::List]);
    let listed = vec![session("s-1", None, None), session("s-2", None, None)];
    let (model, sent) = update(model, TerminalsMsg::Listed(listed.clone()));
    assert!(sent.is_empty());
    assert_eq!(model.sessions, listed);
    assert_eq!(model.session(&id("s-2")), Some(&listed[1]));
    let (model, _) = update(model, TerminalsMsg::Listed(Vec::new()));
    assert_eq!(model.session(&id("s-2")), None);
}

#[test]
fn bringing_back_opens_that_conversation_and_a_refusal_shows_its_sentence() {
    let (model, sent) = update(TerminalsModel::default(), TerminalsMsg::BringBack(id("s-1")));
    assert_eq!(sent, [TerminalsCommand::BringBack(id("s-1"))]);
    let refused = BroughtBack::Refused("It is still running.".to_owned());
    let (model, sent) = update(model, TerminalsMsg::Answered(id("s-1"), refused));
    assert!(sent.is_empty());
    assert_eq!(model.refusal_of(&id("s-1")), Some("It is still running."));
    assert_eq!(model.refusal_of(&id("s-2")), None);
    assert_eq!(model.brought_back, None);
    let (model, sent) = update(
        model,
        TerminalsMsg::Answered(id("s-1"), BroughtBack::Session(id("s-9"))),
    );
    assert_eq!(sent, [TerminalsCommand::List]);
    assert_eq!(model.brought_back, Some(id("s-9")));
    assert_eq!(model.refusal_of(&id("s-1")), None);
}
