//! Which terminal is the default: the Default Terminal Execution specification's lists, then
//! ours.
//!
//! In each configuration folder (the person's, then the system's) the file
//! `<desktop>-xdg-terminals.list` for each of the desktop's names, then `xdg-terminals.list`,
//! names desktop entries, one per line, `#` for a comment; the first that can be run wins. A line
//! that names an action of an entry (`id.desktop:action`) is not read. When no list names a
//! terminal that can be run, temor is the desktop's own; then the terminals found by name; then
//! the first entry in the TerminalEmulator category.

use std::collections::BTreeSet;
use std::io::Read;
use std::path::{Path, PathBuf};

use super::entry::{self, Entry, TERMINAL_CATEGORY};
use super::{Terminal, TerminalSearch};

/// The desktop's own terminal, which is chosen when the person has chosen none.
const OUR_TERMINAL: &str = "org.quire.Temor.desktop";

/// The terminals found by name when no entry names one, in order of preference.
const BY_NAME: [&str; 4] = ["temor", "ghostty", "wezterm", "foot"];

/// The most a list or an entry is read for.
const READ_CAP: u64 = 256 * 1024;

/// The default terminal, or none when there is not one at all.
pub(crate) fn choose(search: &TerminalSearch) -> Option<Terminal> {
    listed_ids(search)
        .into_iter()
        .chain(std::iter::once(OUR_TERMINAL.to_owned()))
        .find_map(|id| entry_of(search, &id))
        .or_else(|| found_by_name(search))
        .or_else(|| in_category(search))
        .map(|entry| Terminal {
            start: entry.start,
            run_arg: entry.run_arg,
        })
}

/// The entry ids the lists name, in the order they are read.
fn listed_ids(search: &TerminalSearch) -> Vec<String> {
    let names = search
        .desktops
        .iter()
        .map(|desktop| format!("{desktop}-xdg-terminals.list"))
        .chain(std::iter::once("xdg-terminals.list".to_owned()))
        .collect::<Vec<_>>();
    search
        .config
        .iter()
        .flat_map(|dir| names.iter().map(move |name| dir.join(name)))
        .filter_map(|path| read_capped(&path))
        .flat_map(|text| ids_in(&text))
        .collect()
}

/// The entry ids a list's text names.
fn ids_in(text: &str) -> Vec<String> {
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#') && !line.contains(':'))
        .map(str::to_owned)
        .collect()
}

/// The entry with desktop id `id` that can be run, from the first data folder that has it.
fn entry_of(search: &TerminalSearch, id: &str) -> Option<Entry> {
    search
        .data
        .iter()
        .map(|dir| dir.join("applications").join(id))
        .find_map(|path| read_capped(&path))
        .and_then(|text| entry::read(&text, &search.programs))
}

/// The first of [`BY_NAME`] that is a program on the path.
fn found_by_name(search: &TerminalSearch) -> Option<Entry> {
    BY_NAME
        .iter()
        .find(|name| entry::is_found(name, &search.programs))
        .map(|name| Entry {
            start: vec![(*name).to_owned()],
            run_arg: "-e".to_owned(),
            categories: Vec::new(),
        })
}

/// The first entry, by desktop id, in the TerminalEmulator category that can be run. An id in
/// the person's folder shadows the same id in a system folder.
fn in_category(search: &TerminalSearch) -> Option<Entry> {
    let mut seen = BTreeSet::new();
    let mut ids: Vec<String> = Vec::new();
    for dir in &search.data {
        for id in entry_ids(&dir.join("applications")) {
            if seen.insert(id.clone()) {
                ids.push(id);
            }
        }
    }
    ids.sort();
    ids.iter()
        .filter_map(|id| entry_of(search, id))
        .find(|entry| entry.categories.iter().any(|c| c == TERMINAL_CATEGORY))
}

/// The names of the `.desktop` files directly in `dir`.
fn entry_ids(dir: &Path) -> Vec<String> {
    std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .filter_map(|file| file.file_name().into_string().ok())
        .filter(|name| name.ends_with(".desktop"))
        .collect()
}

/// The text of the file at `path`, up to [`READ_CAP`] bytes; none when it cannot be read.
fn read_capped(path: &PathBuf) -> Option<String> {
    let mut text = String::new();
    std::fs::File::open(path)
        .ok()?
        .take(READ_CAP)
        .read_to_string(&mut text)
        .ok()?;
    Some(text)
}
