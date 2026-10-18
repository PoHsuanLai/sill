//! The window the sign-in to a helper's app asks for: the terminal this machine uses, running
//! what the companion resolved. The window is the person's: it stays open until they close it,
//! and then the helper is asked to check again.
//!
//! The request carries the command resolved from the list of coding agents, inside the helper's
//! sandbox; that runs in the window with nothing but its own variables, in the helper's folder.
//! A request without the sandboxed command opens nothing: the companion never sends one, and the
//! app is never run unconfined.

use docket_core::helper::{TerminalLine, TerminalRequest};
use prov::HelperId;
use sill_model::companion::HelpersMsg;
use tokio::process::Command;

/// Open the window for `request` and wait until it is closed; the message says how it went.
pub(super) async fn sign_in(request: TerminalRequest) -> HelpersMsg {
    let TerminalRequest { helper, run, .. } = request;
    let Some(folder_and_line) = run.as_deref().and_then(|run| {
        run.confined
            .as_ref()
            .map(|confined| (run.folder.as_deref(), confined))
    }) else {
        return HelpersMsg::SignInUnconfined;
    };
    let wrapper = crate::launch::terminal_wrapper();
    let Some((terminal, lead)) = wrapper.split_first() else {
        return HelpersMsg::NoTerminal;
    };
    let mut command = Command::new(terminal);
    command.args(lead);
    with_confined(&mut command, folder_and_line);
    command
        .stdin(std::process::Stdio::null())
        .kill_on_drop(false);
    match command.spawn() {
        Ok(child) => closed(helper, child).await,
        Err(error) => {
            eprintln!("sill: companion: helpers: the sign-in window did not open: {error}");
            HelpersMsg::NoTerminal
        }
    }
}

/// Add the sandboxed command. It goes through `env -i`, so it gets exactly its own variables while
/// the terminal around it keeps the person's (it needs them to open a window). It starts in the
/// folder when there is one.
fn with_confined(command: &mut Command, (folder, confined): (Option<&str>, &TerminalLine)) {
    command.arg("/usr/bin/env").arg("-i");
    command.args(
        confined
            .env
            .iter()
            .map(|(key, value)| format!("{key}={value}")),
    );
    command.arg(&confined.program).args(&confined.args);
    if let Some(folder) = folder {
        command.current_dir(folder);
    }
}

async fn closed(helper: HelperId, mut child: tokio::process::Child) -> HelpersMsg {
    if let Err(error) = child.wait().await {
        eprintln!("sill: companion: helpers: waiting for the sign-in window: {error}");
    }
    HelpersMsg::TerminalClosed(helper)
}
