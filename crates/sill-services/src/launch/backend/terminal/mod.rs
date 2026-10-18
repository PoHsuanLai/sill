//! The desktop's one way to open a terminal, the counterpart of opening a link in the default
//! browser. Which terminal is the person's default follows the Default Terminal Execution
//! specification (`search.rs`, `choose.rs`): their `xdg-terminals.list` first, then temor (our
//! terminal), then the terminals found by name, then any desktop entry in the TerminalEmulator
//! category. A terminal wraps a command as `<its Exec> <X-TerminalArgExec, else -e> <command>`.
//!
//! Two things use it: `Terminal=true` apps, whose launch is wrapped with [`Terminal::wrapper`],
//! and everything that opens a window running a command, through [`open_in_terminal`].

mod choose;
mod entry;
mod open;
mod search;
#[cfg(test)]
mod tests;

pub(crate) use choose::choose;
pub(crate) use open::{TerminalCommand, open_in_terminal};
pub(crate) use search::TerminalSearch;

/// A terminal chosen: what starts it, and the argument after which it takes a command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Terminal {
    /// The program and the arguments that come before the command's marker.
    pub(crate) start: Vec<String>,
    /// The argument that introduces the command to run (`-e`, unless the entry says otherwise).
    pub(crate) run_arg: String,
}

impl Terminal {
    /// The words that go before a command to make this terminal run it.
    pub(crate) fn wrapper(&self) -> Vec<String> {
        self.start
            .iter()
            .cloned()
            .chain(std::iter::once(self.run_arg.clone()))
            .collect()
    }
}

/// The words that wrap a `Terminal=true` entry's `Exec` on this machine; empty when there is no
/// terminal at all (the caller then runs the bare `Exec`).
pub(crate) fn wrapper(search: &TerminalSearch) -> Vec<String> {
    choose(search)
        .map(|terminal| terminal.wrapper())
        .unwrap_or_default()
}
