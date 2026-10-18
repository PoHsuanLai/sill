//! The window a helper's terminal request asks for: the desktop's default terminal (through the
//! launch service's opener), running what the companion resolved. The window is the person's: it
//! stays open until they close it, and then the screen is told which kind of window it was, so a
//! sign-in asks the helper to check again and a handover asks for the session back.
//!
//! The request carries the command resolved from the list of coding agents, inside the helper's
//! sandbox; that runs in the window with nothing but its own variables, in the helper's folder.
//! A request without the sandboxed command opens nothing: the companion never sends one, and the
//! app is never run unconfined.

use std::collections::BTreeMap;
use std::path::Path;

use docket_core::helper::{TerminalLine, TerminalRequest};
use sill_model::companion::HelpersMsg;

use crate::launch::{TerminalCommand, TerminalSearch, open_in_terminal};

/// Open the window for `request` and wait until it is closed; the message says how it went.
pub(super) async fn open(search: &TerminalSearch, request: TerminalRequest) -> HelpersMsg {
    let TerminalRequest {
        helper,
        run,
        handoff,
        ..
    } = request;
    let Some((folder, confined)) = run.as_deref().and_then(|run| {
        run.confined
            .as_ref()
            .map(|confined| (run.folder.as_deref().map(Path::new), confined))
    }) else {
        return HelpersMsg::SignInUnconfined;
    };
    match open_in_terminal(search, &confined_command(confined), folder) {
        Ok(window) => {
            window.closed().await;
            match handoff {
                Some(_) => HelpersMsg::HandoffClosed(helper),
                None => HelpersMsg::TerminalClosed(helper),
            }
        }
        Err(error) => {
            eprintln!("sill: companion: helpers: the terminal window did not open: {error}");
            HelpersMsg::NoTerminal
        }
    }
}

/// The sandboxed command, through `env -i`, so it gets exactly its own variables while the
/// terminal around it keeps the person's (it needs them to open a window).
fn confined_command(confined: &TerminalLine) -> TerminalCommand {
    TerminalCommand {
        argv: ["/usr/bin/env".to_owned(), "-i".to_owned()]
            .into_iter()
            .chain(
                confined
                    .env
                    .iter()
                    .map(|(key, value)| format!("{key}={value}")),
            )
            .chain(std::iter::once(confined.program.clone()))
            .chain(confined.args.iter().cloned())
            .collect(),
        env: BTreeMap::new(),
    }
}
