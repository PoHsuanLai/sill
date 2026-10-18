//! Opening a terminal window that runs a command, and waiting until the person closes it. The
//! window is the person's: it stays open until they close it. This is where a terminal process
//! is started, by the live service only and never by a test.

use std::collections::BTreeMap;
use std::path::Path;

use tokio::process::{Child, Command};

use super::{TerminalSearch, choose};

/// What a terminal window runs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TerminalCommand {
    /// The program and its arguments.
    pub(crate) argv: Vec<String>,
    /// The variables set over the terminal's own; the command inherits them.
    pub(crate) env: BTreeMap<String, String>,
}

/// Why no window opened.
#[derive(Debug, thiserror::Error)]
pub(crate) enum OpenError {
    /// There is no terminal on this computer.
    #[error("no terminal is installed")]
    NoTerminal,
    /// The terminal would not start.
    #[error("the terminal did not start: {0}")]
    Start(#[from] std::io::Error),
}

/// An open terminal window.
#[derive(Debug)]
pub(crate) struct TerminalWindow {
    child: Child,
}

impl TerminalWindow {
    /// Wait until the window is closed.
    pub(crate) async fn closed(mut self) {
        if let Err(error) = self.child.wait().await {
            eprintln!("sill: terminal: waiting for the window: {error}");
        }
    }
}

/// Open the default terminal running `command`, in `folder` when there is one.
pub(crate) fn open_in_terminal(
    search: &TerminalSearch,
    command: &TerminalCommand,
    folder: Option<&Path>,
) -> Result<TerminalWindow, OpenError> {
    let terminal = choose(search).ok_or(OpenError::NoTerminal)?;
    let wrapper = terminal.wrapper();
    let (program, lead) = wrapper.split_first().ok_or(OpenError::NoTerminal)?;
    let mut process = Command::new(program);
    process
        .args(lead)
        .args(&command.argv)
        .envs(&command.env)
        .stdin(std::process::Stdio::null())
        .kill_on_drop(false);
    if let Some(folder) = folder {
        process.current_dir(folder);
    }
    Ok(TerminalWindow {
        child: process.spawn()?,
    })
}
