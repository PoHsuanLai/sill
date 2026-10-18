//! The window the sign-in to a helper's app asks for: the terminal this machine uses, running the
//! app with what it asked to have added. The window is the person's: it stays open until they
//! close it, and then the helper is asked to check again.
//!
//! The companion's request names the app by its program name and carries only what the app wants
//! added; the sandbox the helper runs in and the app's login folder are not in it, so the app
//! runs here as the person's own program with its own variables over the person's.

use docket_core::helper::TerminalRequest;
use prov::HelperId;
use sill_model::companion::HelpersMsg;
use tokio::process::Command;

/// Open the window for `request` and wait until it is closed; the message says how it went.
pub(super) async fn sign_in(request: TerminalRequest) -> HelpersMsg {
    let TerminalRequest {
        helper,
        program,
        login,
    } = request;
    let wrapper = crate::launch::terminal_wrapper();
    let Some((terminal, lead)) = wrapper.split_first() else {
        return HelpersMsg::NoTerminal;
    };
    let mut command = Command::new(terminal);
    command
        .args(lead)
        .arg(program.as_str())
        .args(&login.args)
        .envs(&login.env)
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

async fn closed(helper: HelperId, mut child: tokio::process::Child) -> HelpersMsg {
    if let Err(error) = child.wait().await {
        eprintln!("sill: companion: helpers: waiting for the sign-in window: {error}");
    }
    HelpersMsg::TerminalClosed(helper)
}
