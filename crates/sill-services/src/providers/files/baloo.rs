//! Baloo (KDE's index) through `baloosearch6`, the one search interface it has (its D-Bus
//! API is for status and control, not queries): `baloosearch6 -l <limit> -d <folder> <text>`
//! with the budget as a timeout, one path per output line. Lane a fills it; [`baloo_args`] and
//! [`parse_paths`] are pure and table-tested.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use sill_launcher::Query;

use super::index::{FileAnswer, FileAsk, FileIndex, IndexFault, IndexKind};

/// The Baloo index.
#[derive(Debug, Default)]
pub struct BalooIndex;

impl FileIndex for BalooIndex {
    fn kind(&self) -> IndexKind {
        IndexKind::Baloo
    }

    fn search(&self, ask: &FileAsk, query: &Query) -> FileAnswer {
        let _ = (ask, query);
        FileAnswer::Unavailable(IndexFault::NotRunning)
    }
}

/// `baloosearch6`'s arguments for `ask` (the text is one argument, never split or quoted by a
/// shell).
pub fn baloo_args(ask: &FileAsk) -> Vec<OsString> {
    let _ = ask;
    todo!("M9 fill: a-files")
}

/// The absolute paths in a search program's output under `under`, one per line: blank lines,
/// relative paths, paths outside `under` and a trailing "Elapsed" line are dropped.
pub fn parse_paths(output: &str, under: &Path) -> Vec<PathBuf> {
    let _ = (output, under);
    todo!("M9 fill: a-files")
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::parse_paths;

    #[test]
    #[ignore = "M9 fill: a-files"]
    fn only_absolute_paths_under_the_folder_survive() {
        let output = "/home/p/a.pdf\n\nrelative/b.txt\n/etc/passwd\n/home/p/dir/c d.md\nElapsed: 1.2 msecs\n";
        assert_eq!(
            parse_paths(output, Path::new("/home/p")),
            vec![PathBuf::from("/home/p/a.pdf"), PathBuf::from("/home/p/dir/c d.md")]
        );
    }
}
