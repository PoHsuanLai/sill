//! Running a search program within a budget (`fd`, `plocate`): its output is read on a thread
//! of its own while this one watches the clock and the query's generation, and the program is killed at the budget with whatever it printed by then.

use std::ffi::{OsStr, OsString};
use std::io::Read;
use std::path::Path;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::time::{Duration, Instant};

use sill_launcher::Query;

use super::index::IndexFault;
use crate::units::Presence;

/// How often a running search looks at the clock and the query's generation.
const TICK: Duration = Duration::from_millis(10);

/// How a program's run ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Ended {
    Exited(ExitStatus),
    /// Killed at the budget.
    OutOfTime,
}

/// What a program printed, and how it ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Ran {
    /// Its standard output, cut after the last complete line when it ran out of time.
    pub text: String,
    pub ended: Ended,
}

/// The names `fd` goes by: its own, and Debian's (`fdfind`, as `fd` is taken there).
pub(super) const FD_NAMES: &[&str] = &["fd", "fdfind"];

/// The first of `names` that is a file in one of `search_path`'s folders (`PATH`'s syntax),
/// asked through `is_file` so a table test needs no disk.
pub(super) fn first_on_path<'a>(
    names: &[&'a str],
    search_path: &OsStr,
    is_file: impl Fn(&Path) -> bool,
) -> Option<&'a str> {
    names
        .iter()
        .copied()
        .find(|name| std::env::split_paths(search_path).any(|dir| is_file(&dir.join(name))))
}

/// Whether any of `names` is on `search_path`.
pub(super) fn on_path_any(
    names: &[&str],
    search_path: &OsStr,
    is_file: impl Fn(&Path) -> bool,
) -> Presence {
    match first_on_path(names, search_path, is_file) {
        Some(_) => Presence::Present,
        None => Presence::Absent,
    }
}

/// Run `program` with `args` (no shell), for at most `budget`, stopping early when `query` is
/// cancelled. The C locale, so a status line reads the same everywhere.
pub(super) fn run_within(
    program: &str,
    args: &[OsString],
    budget: Duration,
    query: &Query,
) -> Result<Ran, IndexFault> {
    let mut child = Command::new(program)
        .args(args)
        .env("LC_ALL", "C")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| match error.kind() {
            std::io::ErrorKind::NotFound => IndexFault::NoProgram,
            _ => IndexFault::Failed,
        })?;
    let chunks = read_on_a_thread(&mut child)?;
    let deadline = Instant::now() + budget;
    let mut bytes = Vec::new();
    loop {
        if query.is_cancelled() {
            stop(&mut child);
            return Err(IndexFault::Cancelled);
        }
        let left = deadline.saturating_duration_since(Instant::now());
        if left.is_zero() {
            stop(&mut child);
            return Ok(Ran {
                text: complete_lines(&bytes),
                ended: Ended::OutOfTime,
            });
        }
        match chunks.recv_timeout(left.min(TICK)) {
            Ok(chunk) => bytes.extend(chunk),
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => break,
        }
    }
    let ended = wait_until(&mut child, deadline);
    Ok(Ran {
        text: match ended {
            Ended::Exited(_) => String::from_utf8_lossy(&bytes).into_owned(),
            Ended::OutOfTime => complete_lines(&bytes),
        },
        ended,
    })
}

/// The child's standard output, chunk by chunk, until it closes.
fn read_on_a_thread(child: &mut Child) -> Result<Receiver<Vec<u8>>, IndexFault> {
    let mut stdout = child.stdout.take().ok_or(IndexFault::Failed)?;
    let (tx, rx) = mpsc::channel();
    std::thread::Builder::new()
        .name("sill-files-read".to_owned())
        .spawn(move || {
            let mut buf = [0u8; 8192];
            while let Ok(n) = stdout.read(&mut buf) {
                if n == 0 || tx.send(buf[..n].to_vec()).is_err() {
                    break;
                }
            }
        })
        .map_err(|_| IndexFault::Failed)?;
    Ok(rx)
}

/// Its output is closed; wait for it to exit, killing it at `deadline`.
fn wait_until(child: &mut Child, deadline: Instant) -> Ended {
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return Ended::Exited(status),
            Ok(None) if Instant::now() < deadline => std::thread::sleep(TICK),
            _ => {
                stop(child);
                return Ended::OutOfTime;
            }
        }
    }
}

fn stop(child: &mut Child) {
    let _ = child.kill();
    let _ = child.wait();
}

/// `bytes` up to and including the last newline: a line cut by the kill is not a path.
fn complete_lines(bytes: &[u8]) -> String {
    let end = bytes
        .iter()
        .rposition(|byte| *byte == b'\n')
        .map_or(0, |i| i + 1);
    String::from_utf8_lossy(&bytes[..end]).into_owned()
}

#[cfg(test)]
mod tests {
    use std::ffi::{OsStr, OsString};
    use std::path::Path;
    use std::time::Duration;

    use sill_launcher::{Cancel, Query};

    use super::{Ended, FD_NAMES, complete_lines, first_on_path, on_path_any, run_within};
    use crate::providers::files::index::IndexFault;
    use crate::units::Presence::{self, Absent, Present};

    #[test]
    fn a_program_is_found_in_any_folder_of_the_path() {
        let exists = |path: &Path| path == Path::new("/usr/bin/fd");
        let cases: &[(&str, &str, Presence)] = &[
            ("fd", "/bin:/usr/bin", Present),
            ("fd", "/bin", Absent),
            ("plocate", "/bin:/usr/bin", Absent),
            ("fd", "", Absent),
        ];
        for &(name, path, want) in cases {
            assert_eq!(
                on_path_any(&[name], OsStr::new(path), exists),
                want,
                "{name} in {path}"
            );
        }
    }

    #[test]
    fn fd_is_found_under_debians_name_too() {
        let debian = |path: &Path| path == Path::new("/usr/bin/fdfind");
        let both = |path: &Path| path.starts_with("/usr/bin");
        let none = |_: &Path| false;
        let path = OsStr::new("/usr/bin");
        assert_eq!(first_on_path(FD_NAMES, path, debian), Some("fdfind"));
        assert_eq!(first_on_path(FD_NAMES, path, both), Some("fd"));
        assert_eq!(first_on_path(FD_NAMES, path, none), None);
    }

    #[test]
    fn a_line_cut_by_the_kill_is_dropped() {
        let cases: &[(&str, &str)] = &[
            ("/a\n/b\n", "/a\n/b\n"),
            ("/a\n/bc", "/a\n"),
            ("/partial", ""),
            ("", ""),
        ];
        for &(bytes, want) in cases {
            assert_eq!(complete_lines(bytes.as_bytes()), want, "{bytes:?}");
        }
    }

    fn args(parts: &[&str]) -> Vec<OsString> {
        parts.iter().map(OsString::from).collect()
    }

    #[test]
    fn a_program_that_finishes_answers_its_whole_output() {
        let query = Query::begin("x", &Cancel::new());
        let ran = run_within(
            "printf",
            &args(&["/a\\n/b"]),
            Duration::from_secs(5),
            &query,
        );
        let ran = ran.unwrap_or_else(|e| panic!("{e:?}"));
        assert_eq!(ran.text, "/a\n/b");
        assert!(matches!(ran.ended, Ended::Exited(status) if status.success()));
    }

    #[test]
    fn a_slow_program_is_killed_at_the_budget_with_what_it_printed() {
        let query = Query::begin("x", &Cancel::new());
        let script = "printf '/early\\n/cut'; sleep 5";
        let ran = run_within(
            "sh",
            &args(&["-c", script]),
            Duration::from_millis(300),
            &query,
        );
        let ran = ran.unwrap_or_else(|e| panic!("{e:?}"));
        assert_eq!(ran.text, "/early\n");
        assert_eq!(ran.ended, Ended::OutOfTime);
    }

    #[test]
    fn a_missing_program_and_a_cancelled_query_are_faults() {
        let cancel = Cancel::new();
        let query = Query::begin("x", &cancel);
        let missing = run_within("sill-no-such-program", &[], Duration::from_secs(1), &query);
        assert_eq!(missing, Err(IndexFault::NoProgram));
        let _newer = Query::begin("xy", &cancel);
        let cancelled = run_within("sleep", &args(&["5"]), Duration::from_secs(5), &query);
        assert_eq!(cancelled, Err(IndexFault::Cancelled));
    }
}
