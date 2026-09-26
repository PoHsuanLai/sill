//! The fallback with no index: `fd` under the home directory (hidden files and `.gitignore`d
//! trees skipped, as `fd` does by default), else `plocate` filtered to the home directory, each
//! killed at the budget with whatever it printed by then. Lane a fills it; [`fd_args`] and
//! [`plocate_args`] are pure and table-tested (output lines go through
//! `baloo::parse_paths`).

use std::ffi::OsString;

use sill_launcher::Query;

use super::index::{FileAnswer, FileAsk, FileIndex, IndexFault, IndexKind};

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
        let _ = (ask, query);
        FileAnswer::Unavailable(IndexFault::NoProgram)
    }
}

/// `fd`'s arguments: a case-insensitive fixed-string match on the name, absolute paths, at most
/// `ask.limit` results, under `ask.under`.
pub fn fd_args(ask: &FileAsk) -> Vec<OsString> {
    let _ = ask;
    todo!("M9 fill: a-files")
}

/// `plocate`'s arguments: case-insensitive, basename only, at most `ask.limit` results (the
/// home-directory filter is applied to its output).
pub fn plocate_args(ask: &FileAsk) -> Vec<OsString> {
    let _ = ask;
    todo!("M9 fill: a-files")
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;
    use std::path::PathBuf;
    use std::time::Duration;

    use super::{fd_args, plocate_args};
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
    #[ignore = "M9 fill: a-files"]
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
}
