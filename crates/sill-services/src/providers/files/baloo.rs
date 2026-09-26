//! Baloo (KDE's index) through `baloosearch6`, the one search interface it has (its D-Bus
//! API is for status and control, not queries): `baloosearch6 -l <limit> -d <folder> <text>`
//! with the budget as a timeout, one path per output line. Lane a fills it; [`baloo_args`] and
//! [`parse_paths`] are pure and table-tested.

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::time::Duration;

use sill_launcher::{Cancel, Query};

use super::choice::IndexState;
use super::index::{FileAnswer, FileAsk, FileIndex, HitLimit, IndexFault, IndexKind};
use super::program::{Ended, Ran, run_within};
use super::stat::hit_at;

/// How long `balooctl6 status` may take when detecting.
const STATUS_BUDGET: Duration = Duration::from_secs(1);

/// The Baloo index.
#[derive(Debug, Default)]
pub struct BalooIndex;

impl FileIndex for BalooIndex {
    fn kind(&self) -> IndexKind {
        IndexKind::Baloo
    }

    fn search(&self, ask: &FileAsk, query: &Query) -> FileAnswer {
        match run_within("baloosearch6", &baloo_args(ask), ask.budget, query) {
            Ok(ran) => answer_of(ran, &ask.under, ask.limit),
            Err(fault) => FileAnswer::Unavailable(fault),
        }
    }
}

/// `baloosearch6`'s arguments for `ask` (the text is one argument, never split or quoted by a
/// shell).
pub fn baloo_args(ask: &FileAsk) -> Vec<OsString> {
    vec![
        OsString::from("-l"),
        OsString::from(ask.limit.0.to_string()),
        OsString::from("-d"),
        OsString::from(&ask.under),
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
pub(super) fn answer_of(ran: Ran, under: &Path, limit: HitLimit) -> FileAnswer {
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

/// Whether Baloo is running, enabled but stopped, or disabled, from `balooctl6 status` (it
/// prints that it is disabled and exits 0, so the words decide).
pub(super) fn baloo_state() -> IndexState {
    let query = Query::begin("", &Cancel::new());
    let args = [OsString::from("status")];
    match run_within("balooctl6", &args, STATUS_BUDGET, &query) {
        Ok(ran) => baloo_status(&ran.text),
        Err(_) => IndexState::Absent,
    }
}

/// `balooctl6 status`'s output (C locale) read as a state. "is not running" is checked before
/// "is running", which it contains.
fn baloo_status(output: &str) -> IndexState {
    let output = output.to_lowercase();
    if output.trim().is_empty() || output.contains("disabled") {
        IndexState::Absent
    } else if output.contains("is not running") {
        IndexState::Dormant
    } else if output.contains("is running") {
        IndexState::Running
    } else {
        IndexState::Dormant
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;
    use std::path::{Path, PathBuf};
    use std::time::Duration;

    use super::{baloo_args, baloo_status, parse_paths};
    use crate::providers::files::choice::IndexState::{self, Absent, Dormant, Running};
    use crate::providers::files::index::{FileAsk, HitLimit};

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

    #[test]
    fn the_text_is_one_argument_after_the_options() {
        let ask = FileAsk {
            text: "-x tax 2025".to_owned(),
            under: PathBuf::from("/home/p"),
            limit: HitLimit(40),
            budget: Duration::from_millis(150),
        };
        let want: Vec<OsString> = ["-l", "40", "-d", "/home/p", "--", "-x tax 2025"]
            .iter()
            .map(OsString::from)
            .collect();
        assert_eq!(baloo_args(&ask), want);
    }

    #[test]
    fn baloo_is_running_stopped_or_disabled() {
        let cases: &[(&str, IndexState)] = &[
            (
                "Baloo is currently disabled. To enable, please run balooctl enable\n",
                Absent,
            ),
            (
                "Baloo File Indexer is running\nIndexer state: Idle\n",
                Running,
            ),
            ("Baloo File Indexer is not running\n", Dormant),
            ("something else\n", Dormant),
            ("", Absent),
        ];
        for &(output, want) in cases {
            assert_eq!(baloo_status(output), want, "{output:?}");
        }
    }
}
