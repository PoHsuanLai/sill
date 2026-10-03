//! The Terminal module on the real control center, on the fake intents service: nothing while the
//! terminal has no grant and is not paused; each standing grant as a row with its Revoke, which
//! sends `RevokeGrant`; a paused terminal's reason and "Resume the terminal", which sends
//! `ResumeTerminal`; and opening the panel asks for the grants to be read again. Nothing here
//! reaches intentd.

mod cc_support;

use cc_support::details::{center, ms, press};
use ds_harness::{Driver, Query};
use sill_model::intents::terminal::{Breaker, ControlReach, Terminal};
use sill_model::intents::{ActionRef, Intents, IntentsCommand, IntentsLink};

fn grant(app: &str, name: &str) -> ActionRef {
    serde_json::from_str(&format!(r#"{{"app":"{app}","name":"{name}"}}"#))
        .unwrap_or_else(|e| panic!("{e}"))
}

fn seat(terminal: Terminal) -> Intents {
    Intents {
        link: IntentsLink::Serving,
        terminal,
        ..Intents::default()
    }
}

fn sent(running: &mut cc_support::live::Running) -> Vec<IntentsCommand> {
    std::iter::from_fn(|| running.feeds.intents_commands.try_recv().ok()).collect()
}

#[test]
fn nothing_is_drawn_while_the_terminal_has_nothing_to_say() {
    let mut running = center(|_| {});
    assert_eq!(running.harness.count(".sill-cc-terminal"), 0);
    assert_eq!(
        sent(&mut running),
        vec![IntentsCommand::RefreshGrants],
        "opening the panel reads the grants again"
    );
}

#[test]
fn each_grant_is_a_row_and_revoke_sends_that_grant() {
    let mail = grant("org.quire.Mail", "mail.thread.archive");
    let shell = grant("org.quire.Shell", "shell.dock.pin");
    let mut running = center(|feeds| {
        feeds.intents.send_replace(seat(Terminal {
            grants: vec![mail.clone(), shell.clone()],
            reach: ControlReach::Reached,
            ..Terminal::default()
        }));
    });
    sent(&mut running);
    let h = &mut running.harness;
    assert_eq!(h.count(".sill-cc-terminal-row"), 2);
    assert_eq!(
        h.text_of(".sill-cc-terminal-row:nth-child(1) .sill-cc-terminal-action")
            .as_deref(),
        Some("Thread archive")
    );
    assert_eq!(
        h.text_of(".sill-cc-terminal-row:nth-child(1) .sill-cc-terminal-app")
            .as_deref(),
        Some("in Mail")
    );
    assert_eq!(
        h.text_of(".sill-cc-terminal-row:nth-child(2) .sill-cc-terminal-action")
            .as_deref(),
        Some("Dock pin")
    );
    assert_eq!(h.count("#terminal-resume"), 0, "not paused");
    press(h, ".sill-cc-terminal-row:nth-child(2) .ds-button");
    h.advance(ms(40));
    assert_eq!(sent(&mut running), vec![IntentsCommand::RevokeGrant(shell)]);
}

#[test]
fn a_paused_terminal_says_why_and_resumes_on_the_button() {
    let mut running = center(|feeds| {
        feeds.intents.send_replace(seat(Terminal {
            breaker: Breaker::Tripped,
            reach: ControlReach::Reached,
            ..Terminal::default()
        }));
    });
    sent(&mut running);
    let h = &mut running.harness;
    assert_eq!(h.count(".sill-cc-terminal-paused"), 1);
    assert_eq!(
        h.text_of("#terminal-resume").as_deref(),
        Some("Resume the terminal")
    );
    assert_eq!(h.count(".sill-cc-terminal-row"), 0);
    press(h, "#terminal-resume");
    h.advance(ms(40));
    assert_eq!(sent(&mut running), vec![IntentsCommand::ResumeTerminal]);
    // The service says it is running again: the module has nothing left to say.
    running
        .feeds
        .intents
        .send_replace(seat(Terminal::default()));
    running.harness.advance(ms(100));
    assert_eq!(running.harness.count(".sill-cc-terminal"), 0);
}
