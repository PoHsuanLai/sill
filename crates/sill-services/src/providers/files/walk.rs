//! The fallback with no index: `fd` under the home directory (hidden files and `.gitignore`d
//! trees skipped, as `fd` does by default), else `plocate` filtered to the home directory, each
//! killed at the budget with whatever it printed by then. [`fd_args`], [`plocate_args`] and
//! [`parse_paths`] are pure and table-tested.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use sill_launcher::Query;

use super::index::{FileAnswer, FileAsk, FileIndex, HitLimit, IndexFault, IndexKind};
use super::program::{Ended, FD_NAMES, Ran, first_on_path, run_within};
use super::stat::hit_at;

/// Which program walks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Walker {
    Fd,
    Plocate,
}

/// The walking index.
#[derive(Debug)]
pub struct WalkIndex {
    pub walker: Walker,
}

impl FileIndex for WalkIndex {
    fn kind(&self) -> IndexKind {
        match self.walker {
            Walker::Fd => IndexKind::Fd,
            Walker::Plocate => IndexKind::Plocate,
        }
    }

    fn search(&self, ask: &FileAsk, query: &Query) -> FileAnswer {
        let (program, args) = match self.walker {
            Walker::Fd => (fd_program(), fd_args(ask)),
            Walker::Plocate => ("plocate", plocate_args(ask)),
        };
        match run_within(program, &args, ask.budget, query) {
            Ok(ran) => answer_of(ran, &ask.under, ask.limit),
            Err(fault) => FileAnswer::Unavailable(fault),
        }
    }
}

/// `fd` under whichever name this machine has it (`fdfind` on Debian); `fd` when neither is
/// found, so the run fails as a missing program.
fn fd_program() -> &'static str {
    let search_path = std::env::var_os("PATH").unwrap_or_default();
    first_on_path(FD_NAMES, &search_path, Path::is_file).unwrap_or("fd")
}

/// `fd`'s arguments: a case-insensitive fixed-string match on the name, absolute paths, at most
/// `ask.limit` results, under `ask.under`.
pub fn fd_args(ask: &FileAsk) -> Vec<OsString> {
    let mut args: Vec<OsString> = [
        "--fixed-strings",
        "--ignore-case",
        "--absolute-path",
        "--max-results",
    ]
    .iter()
    .map(OsString::from)
    .collect();
    args.push(OsString::from(ask.limit.0.to_string()));
    args.push(OsString::from("--"));
    args.push(OsString::from(&ask.text));
    args.push(OsString::from(&ask.under));
    args
}

/// `plocate`'s arguments: case-insensitive, basename only, at most `ask.limit` results (the
/// home-directory filter is applied to its output).
pub fn plocate_args(ask: &FileAsk) -> Vec<OsString> {
    vec![
        OsString::from("--ignore-case"),
        OsString::from("--basename"),
        OsString::from("--limit"),
        OsString::from(ask.limit.0.to_string()),
        OsString::from("--"),
        OsString::from(&ask.text),
    ]
}

/// The absolute paths in a search program's output under `under`, one per line: blank lines,
/// relative paths, paths outside `under` and a trailing "Elapsed" line are dropped.
pub fn parse_paths(output: &str, under: &Path) -> Vec<PathBuf> {
    output
        .lines()
        .map(Path::new)
        .filter(|path| path.is_absolute() && path.starts_with(under))
        .map(Path::to_path_buf)
        .collect()
}

/// A search program's run as an answer: every path under `under` that still exists, at most
/// `limit`; `Partial` when it was killed at the budget. A program that exited badly still
/// printed paths worth showing (`fd` exits 1 after a folder it may not read; `plocate` exits 1
/// when nothing matched).
fn answer_of(ran: Ran, under: &Path, limit: HitLimit) -> FileAnswer {
    let hits = parse_paths(&ran.text, under)
        .into_iter()
        .filter_map(hit_at)
        .take(usize::from(limit.0))
        .collect();
    match ran.ended {
        Ended::Exited(status) if status.code().is_none() => {
            FileAnswer::Unavailable(IndexFault::Failed)
        }
        Ended::Exited(_) => FileAnswer::Hits(hits),
        Ended::OutOfTime => FileAnswer::Partial(hits),
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;
    use std::path::{Path, PathBuf};
    use std::time::Duration;

    use super::{fd_args, parse_paths, plocate_args};
    use crate::providers::files::index::{FileAsk, HitLimit};

    fn ask(text: &str) -> FileAsk {
        FileAsk {
            text: text.to_owned(),
            under: PathBuf::from("/home/p"),
            limit: HitLimit(20),
            budget: Duration::from_millis(150),
        }
    }

    fn argv(parts: &[&str]) -> Vec<OsString> {
        parts.iter().map(OsString::from).collect()
    }

    #[test]
    fn the_text_is_one_fixed_string_argument() {
        assert_eq!(
            fd_args(&ask("a b; rm")),
            argv(&[
                "--fixed-strings",
                "--ignore-case",
                "--absolute-path",
                "--max-results",
                "20",
                "--",
                "a b; rm",
                "/home/p",
            ])
        );
        assert_eq!(
            plocate_args(&ask("-rf")),
            argv(&["--ignore-case", "--basename", "--limit", "20", "--", "-rf"])
        );
    }

    #[test]
    fn only_absolute_paths_under_the_folder_survive() {
        let output = "/home/p/a.pdf\n\nrelative/b.txt\n/etc/passwd\n/home/p/dir/c d.md\nElapsed: 1.2 msecs\n";
        assert_eq!(
            parse_paths(output, Path::new("/home/p")),
            vec![
                PathBuf::from("/home/p/a.pdf"),
                PathBuf::from("/home/p/dir/c d.md")
            ]
        );
    }

    #[test]
    fn a_neighbour_of_the_folder_is_not_under_it() {
        let output = "/home/pat/a.pdf\n/home/p/b.pdf\n/home/p\n";
        assert_eq!(
            parse_paths(output, Path::new("/home/p")),
            vec![PathBuf::from("/home/p/b.pdf"), PathBuf::from("/home/p")]
        );
    }
}
