//! A terminal's desktop entry, read for what opening a terminal needs: the command that starts
//! it, the argument that introduces a command to run in it, and its categories.

use std::path::{Path, PathBuf};

use ds_helpers::{DESKTOP_ENTRY_CAP, DesktopFile};
use sill_model::apps::{ExecArgs, expand_exec};

/// The argument that introduces a command when the entry names none.
const DEFAULT_RUN_ARG: &str = "-e";

/// The entry key that names the argument that introduces a command.
const RUN_ARG_KEY: &str = "X-TerminalArgExec";

/// The category every terminal's entry lists.
pub(super) const TERMINAL_CATEGORY: &str = "TerminalEmulator";

/// What an entry says about starting its terminal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Entry {
    /// `Exec`, as words.
    pub(super) start: Vec<String>,
    /// `X-TerminalArgExec`, else `-e`.
    pub(super) run_arg: String,
    /// `Categories`.
    pub(super) categories: Vec<String>,
}

/// The entry in `text`, when it is one that can be run: an application that is not hidden,
/// whose `TryExec` (if any) and whose program are found.
pub(super) fn read(text: &str, programs: &[PathBuf]) -> Option<Entry> {
    let file = DesktopFile::parse(text, DESKTOP_ENTRY_CAP).ok()?;
    let entry = file.entry()?;
    if entry.is_true("Hidden")
        || entry
            .string("Type")
            .is_some_and(|kind| kind != "Application")
    {
        return None;
    }
    if entry
        .string("TryExec")
        .is_some_and(|bin| !is_found(&bin, programs))
    {
        return None;
    }
    let start = expand_exec(&entry.string("Exec")?, &ExecArgs::default()).ok()?;
    if !is_found(start.first()?, programs) {
        return None;
    }
    Some(Entry {
        start,
        run_arg: entry
            .string(RUN_ARG_KEY)
            .map(|arg| arg.trim().to_owned())
            .filter(|arg| !arg.is_empty())
            .unwrap_or_else(|| DEFAULT_RUN_ARG.to_owned()),
        categories: entry.list("Categories"),
    })
}

/// Whether `program`, an absolute path or a name, is an executable file (a name is looked for in
/// `programs`).
pub(super) fn is_found(program: &str, programs: &[PathBuf]) -> bool {
    let path = Path::new(program);
    match path.is_absolute() {
        true => is_executable(path),
        false => programs.iter().any(|dir| is_executable(&dir.join(program))),
    }
}

fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    std::fs::metadata(path)
        .is_ok_and(|meta| meta.is_file() && meta.permissions().mode() & 0o111 != 0)
}
