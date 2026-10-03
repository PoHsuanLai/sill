use docket_core::ActionRef;
use prov::ActionName;

use super::{Breaker, ControlReach, Terminal, TerminalEvent, grant_words, step};
use crate::intents::tests::app;

fn grant(app_name: &str, action: &str) -> ActionRef {
    ActionRef {
        app: app(app_name),
        name: ActionName::parse(action).unwrap_or_else(|e| panic!("{action}: {e}")),
    }
}

#[test]
fn grants_load_sorted_without_repeats_and_mark_the_control_reached() {
    let a = grant("org.quire.Mail", "mail.thread.archive");
    let b = grant("org.quire.Shell", "shell.lock");
    let terminal = step(
        Terminal::default(),
        TerminalEvent::Loaded(vec![b.clone(), a.clone(), b.clone()]),
    );
    assert_eq!(terminal.grants, vec![a, b]);
    assert_eq!(terminal.reach, ControlReach::Reached);
}

#[test]
fn an_unreachable_intentd_keeps_what_was_shown_and_says_so() {
    let a = grant("org.quire.Mail", "mail.thread.archive");
    let loaded = step(Terminal::default(), TerminalEvent::Loaded(vec![a.clone()]));
    let after = step(loaded, TerminalEvent::Unreachable);
    assert_eq!(after.grants, vec![a]);
    assert_eq!(after.reach, ControlReach::Unreachable);
}

#[test]
fn a_revoked_grant_leaves_the_list_and_the_others_stay() {
    let a = grant("org.quire.Mail", "mail.thread.archive");
    let b = grant("org.quire.Shell", "shell.lock");
    let loaded = step(
        Terminal::default(),
        TerminalEvent::Loaded(vec![a.clone(), b.clone()]),
    );
    let after = step(loaded, TerminalEvent::Revoked(a));
    assert_eq!(after.grants, vec![b]);
}

#[test]
fn the_breaker_trips_and_resumes() {
    let tripped = step(
        Terminal::default(),
        TerminalEvent::Breaker(Breaker::Tripped),
    );
    assert_eq!(tripped.breaker, Breaker::Tripped);
    let resumed = step(tripped, TerminalEvent::Breaker(Breaker::Running));
    assert_eq!(resumed.breaker, Breaker::Running);
}

#[test]
fn a_grant_is_read_as_its_action_in_its_app() {
    let cases = [
        (
            ("org.quire.Mail", "mail.thread.archive"),
            ("Thread archive", "Mail"),
        ),
        (("org.quire.Shell", "shell.lock"), ("Lock", "Shell")),
        (
            ("org.quire.Shell", "shell.workspace.switch"),
            ("Workspace switch", "Shell"),
        ),
        // A name that does not start with its app's slug is shown whole.
        (("org.quire.Mail", "other.thing"), ("Other thing", "Mail")),
    ];
    for ((app_name, action), (words, in_app)) in cases {
        assert_eq!(
            grant_words(&grant(app_name, action)),
            (words.to_owned(), in_app.to_owned()),
            "{action}"
        );
    }
}
