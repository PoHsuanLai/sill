//! The window the sign-in to a helper's app asks for: the terminal this machine uses, running
//! what the companion resolved. The window is the person's: it stays open until they close it,
//! and then the helper is asked to check again.
//!
//! The request carries the command resolved from the list of coding agents. When it has the
//! command inside the helper's sandbox, that runs in the window with nothing but its own
//! variables, in the helper's folder. Otherwise the plain command runs with its variables over the
//! person's. A request that carries neither names the app by its program name and what to add,
//! and the app runs as the person's own program.

use docket_core::helper::{TerminalLine, TerminalRequest, TerminalRun};
use prov::HelperId;
use sill_model::companion::HelpersMsg;
use tokio::process::Command;

/// Open the window for `request` and wait until it is closed; the message says how it went.
pub(super) async fn sign_in(request: TerminalRequest) -> HelpersMsg {
    let TerminalRequest {
        helper,
        program,
        login,
        run,
    } = request;
    let wrapper = crate::launch::terminal_wrapper();
    let Some((terminal, lead)) = wrapper.split_first() else {
        return HelpersMsg::NoTerminal;
    };
    let mut command = Command::new(terminal);
    command.args(lead);
    match run.as_deref() {
        Some(run) => with_run(&mut command, run),
        None => {
            command
                .arg(program.as_str())
                .args(&login.args)
                .envs(&login.env);
        }
    }
    command.stdin(std::process::Stdio::null()).kill_on_drop(false);
    match command.spawn() {
        Ok(child) => closed(helper, child).await,
        Err(error) => {
            eprintln!("sill: companion: helpers: the sign-in window did not open: {error}");
            HelpersMsg::NoTerminal
        }
    }
}

/// Add what `run` starts. The sandboxed command goes through `env -i`, so it gets exactly its own
/// variables while the terminal around it keeps the person's (it needs them to open a window);
/// the plain command has its variables set over the terminal's. Either starts in the folder when
/// there is one.
fn with_run(command: &mut Command, run: &TerminalRun) {
    match &run.confined {
        Some(confined) => {
            command.arg("/usr/bin/env").arg("-i");
            command.args(confined.env.iter().map(|(key, value)| format!("{key}={value}")));
            command.arg(&confined.program).args(&confined.args);
        }
        None => {
            let TerminalLine { program, args, env } = &run.plain;
            command.arg(program).args(args).envs(env);
        }
    }
    if let Some(folder) = &run.folder {
        command.current_dir(folder);
    }
}

async fn closed(helper: HelperId, mut child: tokio::process::Child) -> HelpersMsg {
    if let Err(error) = child.wait().await {
        eprintln!("sill: companion: helpers: waiting for the sign-in window: {error}");
    }
    HelpersMsg::TerminalClosed(helper)
}
