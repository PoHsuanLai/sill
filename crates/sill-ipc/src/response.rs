//! What the daemon answers.

use serde::{Deserialize, Serialize};

use crate::debug::{LauncherRows, SurfaceStats};
use crate::status::StatusReport;

/// The one reply to one [`crate::Request`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum Response {
    /// The answer to `Ping`.
    Pong,
    /// The request was carried out.
    Done,
    /// The request was understood and not carried out.
    Failed(Failure),
    /// The answer to `Status`.
    Status(StatusReport),
    /// The answer to `Debug(Stats)`, one entry per surface.
    Stats(Vec<SurfaceStats>),
    /// The answer to `Debug(LauncherRows)`.
    LauncherRows(LauncherRows),
}

/// Why a request was not carried out, and what the caller can do about it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Failure {
    pub kind: FailureKind,
    /// For a person reading the command line's stderr.
    pub message: String,
}

/// The decision a failure carries (CONVENTIONS §7: "an error exists so a caller can act").
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FailureKind {
    /// A debug request named a surface that does not exist; retrying will not help.
    UnknownSurface,
    /// This build or this compositor cannot do it (no `debug` feature, no layer-shell, ...).
    Unsupported,
    /// The shell will not do this now (`quit` while the session is locked); the message says why.
    Refused,
    /// The daemon did not finish within the reply timeout (2 s); it may still finish.
    Timeout,
    /// Anything else; the message says what.
    Internal,
}
