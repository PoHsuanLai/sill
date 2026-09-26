//! The real [`Authenticator`]: one `sill auth-check` child per attempt (swaylock's shape, module
//! doc), spoken to with [`super::protocol`].

use std::path::PathBuf;
use std::time::Duration;

use super::authenticator::Authenticator;
use super::types::{Attempt, Verdict};

/// How long one check may run before its child is killed (`Unavailable(TimedOut)`): past
/// `pam_unix`'s 2 s fail delay and a slow directory lookup, short of a person giving up.
pub const CHECK_TIMEOUT: Duration = Duration::from_secs(15);

/// Runs `program args` with the request on stdin and reads the verdict from stdout.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PamHelper {
    pub program: PathBuf,
    pub args: Vec<String>,
    pub timeout: Duration,
}

impl PamHelper {
    /// This executable's own `auth-check` subcommand (`sill auth-check`).
    pub fn this_exe() -> PamHelper {
        PamHelper {
            program: std::env::current_exe().unwrap_or_else(|_| PathBuf::from("sill")),
            args: vec!["auth-check".to_owned()],
            timeout: CHECK_TIMEOUT,
        }
    }
}

impl Authenticator for PamHelper {
    /// Resolves the service ([`super::pam::service`]), writes
    /// [`super::protocol::encode_request`] to the child's stdin and closes it, reads its lines
    /// until it exits or `timeout` passes (then kills it), and folds them with
    /// [`super::protocol::verdict_of`]; a child that cannot start is `Unavailable(NoHelper)`.
    fn check(&self, attempt: &Attempt) -> Verdict {
        todo!("M11 fill: a-lock ({attempt:?})")
    }
}
