//! What the daemon answers.

use serde::{Deserialize, Serialize};

use crate::request::SurfaceName;

/// The one reply to one [`crate::Request`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum Response {
    /// The answer to `Ping`.
    Pong,
    /// The request was carried out.
    Done,
    /// The answer to `Debug(Stats)`, one entry per surface.
    Stats(Vec<SurfaceStats>),
    /// The request was understood and not carried out.
    Failed(Failure),
}

/// Why a request was not carried out, and what the caller can do about it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Failure {
    pub kind: FailureKind,
    /// For a person reading the command line's stderr.
    pub message: String,
}

/// The decision a failure carries (CONVENTIONS §0 "errors carry a decision").
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FailureKind {
    /// A debug request named a surface that does not exist; retrying will not help.
    UnknownSurface,
    /// This build or this compositor cannot do it (no `debug` feature, no layer-shell, ...).
    Unsupported,
    /// The daemon did not finish within the reply timeout (2 s); it may still finish.
    Timeout,
    /// Anything else; the message says what.
    Internal,
}

/// One surface's frame statistics, the IPC mirror of shell-host's `FrameStats` (this crate may
/// not depend on shell-host: `scripts/check-boundary.sh`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SurfaceStats {
    pub surface: SurfaceName,
    /// Frames presented since the surface was created.
    pub frames: u64,
    /// Frame callbacks received.
    pub callbacks: u64,
    /// `wl_surface.commit`s; idle means this number stops moving.
    pub commits: u64,
    /// Frames that were due but not presented.
    pub skipped: u64,
    /// Watchdog expiries.
    pub watchdog_fired: u64,
    /// Trigger-to-present time over the samples since the previous report.
    pub work: Micros,
    /// Present-to-present time over the same samples.
    pub interval: Micros,
    /// How many frames the two percentile sets cover.
    pub samples: u32,
    /// The daemon's resident set size, KiB.
    pub vm_rss_kib: u64,
}

/// Nearest-rank percentiles of a set of durations, in microseconds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Micros {
    pub p50_us: u32,
    pub p95_us: u32,
    pub max_us: u32,
}
