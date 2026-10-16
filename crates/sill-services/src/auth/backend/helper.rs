//! The real [`Authenticator`]: one `sill auth-check` child per attempt (swaylock's shape, module
//! doc), spoken to with [`sill_model::auth::protocol`].

use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdout, Command, Stdio};
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::time::{Duration, Instant};

use sill_model::auth::protocol::{HelperLine, HelperRequest, ProtocolError};
use sill_model::auth::{Attempt, AuthFault, Outcome, Verdict, protocol};

use super::pam;
use crate::env::Authenticator;

/// How long one check may run before its child is killed (`Unavailable(TimedOut)`): past
/// `pam_unix`'s 2 s fail delay and a slow directory lookup, short of a person giving up.
pub const CHECK_TIMEOUT: Duration = Duration::from_secs(15);

/// The running program's own image, as the kernel names it for the calling process.
const THIS_EXE: &str = "/proc/self/exe";

/// Runs `program args` with the request on stdin and reads the verdict from stdout.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PamHelper {
    pub program: PathBuf,
    pub args: Vec<String>,
    pub timeout: Duration,
}

impl PamHelper {
    /// This running executable's own `auth-check` subcommand (`sill auth-check`). Started through
    /// `/proc/self/exe`, which names the program image this process runs even after an update has
    /// replaced or removed its file, so the daemon and its helper are always the same build.
    pub fn this_exe() -> PamHelper {
        PamHelper {
            program: PathBuf::from(THIS_EXE),
            args: vec!["auth-check".to_owned()],
            timeout: CHECK_TIMEOUT,
        }
    }
}

impl Authenticator for PamHelper {
    /// Resolves the service ([`super::pam::service`]), writes
    /// `sill_model::auth::encode_request` to the child's stdin and closes it, reads its lines
    /// until it exits or `timeout` passes (then kills it), and folds them with
    /// `sill_model::auth::verdict_of`; a child that cannot start is `Unavailable(NoHelper)`.
    /// The encoded request's bytes are zeroed once written; the secret is never logged.
    fn check(&self, attempt: &Attempt) -> Verdict {
        let request = HelperRequest {
            service: pam::service(Path::exists),
            attempt: attempt.clone(),
        };
        let Ok(mut child) = Command::new(&self.program)
            .args(&self.args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
        else {
            return Verdict::plain(Outcome::Unavailable(AuthFault::NoHelper));
        };
        hand_over(&mut child, &request);
        drop(request);
        let Some(stdout) = child.stdout.take() else {
            end(&mut child);
            return Verdict::plain(Outcome::Unavailable(AuthFault::HelperFailed));
        };
        let lines = read_lines(stdout);
        match collect(&lines, Instant::now() + self.timeout) {
            Collected::Whole(lines) => {
                let _ = child.wait();
                protocol::verdict_of(lines)
            }
            Collected::TimedOut => {
                end(&mut child);
                Verdict::plain(Outcome::Unavailable(AuthFault::TimedOut))
            }
        }
    }
}

/// Write the request on the child's stdin and close it; the encoded bytes are zeroed once
/// written. A child that exits before reading everything (a broken pipe) is heard from, or not,
/// on its stdout.
fn hand_over(child: &mut Child, request: &HelperRequest) {
    let mut bytes = protocol::encode_request(request);
    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(&bytes);
        let _ = stdin.flush();
    }
    bytes.iter_mut().for_each(|byte| *byte = 0);
    std::hint::black_box(&bytes);
}

/// Kill the child (it may already be gone) and reap it.
fn end(child: &mut Child) {
    let _ = child.kill();
    let _ = child.wait();
}

/// The child's stdout, a line at a time, from a thread of its own (so the wait can time out);
/// the channel closes at end of file.
fn read_lines(stdout: ChildStdout) -> Receiver<Result<HelperLine, ProtocolError>> {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        for line in BufReader::new(stdout).lines() {
            let parsed = match line {
                Ok(text) => protocol::parse_line(&text),
                Err(_) => Err(ProtocolError::Line(String::new())),
            };
            if tx.send(parsed).is_err() {
                break;
            }
        }
    });
    rx
}

/// Every line until end of file, or the deadline.
enum Collected {
    Whole(Vec<Result<HelperLine, ProtocolError>>),
    TimedOut,
}

fn collect(lines: &Receiver<Result<HelperLine, ProtocolError>>, deadline: Instant) -> Collected {
    let mut got = Vec::new();
    loop {
        let left = deadline.saturating_duration_since(Instant::now());
        match lines.recv_timeout(left) {
            Ok(line) => got.push(line),
            Err(RecvTimeoutError::Disconnected) => return Collected::Whole(got),
            Err(RecvTimeoutError::Timeout) => return Collected::TimedOut,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::PamHelper;

    #[test]
    fn the_helper_is_the_running_programs_own_image() {
        let helper = PamHelper::this_exe();
        assert_eq!(helper.program, Path::new("/proc/self/exe"));
        assert_eq!(helper.args, vec!["auth-check".to_owned()]);
        assert!(
            helper.program.exists(),
            "the kernel names this program's image"
        );
    }
}
