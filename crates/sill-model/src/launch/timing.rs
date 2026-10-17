//! When a launch was asked for and when its app's process existed, and the one line that says so.

use serde::{Deserialize, Serialize};
use sill_launcher::AppId;

use super::{FailureCause, ScopeUnit};

/// A point on the monotonic clock (`CLOCK_MONOTONIC`), in nanoseconds: the clock a shell script
/// reads with `time.clock_gettime_ns(time.CLOCK_MONOTONIC)`, so two programs' stamps compare.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Stamp(pub u64);

impl Stamp {
    /// The stamp of a clock reading in whole seconds and the nanoseconds past them.
    pub fn of_reading(seconds: u64, nanos: u64) -> Stamp {
        Stamp(seconds.saturating_mul(1_000_000_000).saturating_add(nanos))
    }
}

/// A process id.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Pid(pub u32);

/// One launch that reached an app: when it was asked for and when the app's process existed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LaunchTiming {
    pub app: AppId,
    /// The scope the app runs in; none when the app was started by D-Bus activation.
    pub unit: Option<ScopeUnit>,
    /// When the request reached the launch code.
    pub requested_at: Stamp,
    /// When the app's process existed (the scope's main process), or when the activation call
    /// returned.
    pub spawned_at: Stamp,
    /// The app's process; none for D-Bus activation, which starts no child of ours.
    pub pid: Option<Pid>,
}

/// How one launch request ended, for whoever is waiting to hear.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum LaunchDone {
    Spawned(LaunchTiming),
    Refused {
        app: AppId,
        requested_at: Stamp,
        cause: FailureCause,
    },
}

impl LaunchDone {
    /// The app the request was for.
    pub fn app(&self) -> &AppId {
        match self {
            LaunchDone::Spawned(timing) => &timing.app,
            LaunchDone::Refused { app, .. } => app,
        }
    }

    /// When the request reached the launch code.
    pub fn requested_at(&self) -> Stamp {
        match self {
            LaunchDone::Spawned(timing) => timing.requested_at,
            LaunchDone::Refused { requested_at, .. } => *requested_at,
        }
    }
}

/// The first of `done` that answers a request for `app` made at or after `asked_at`.
pub fn answer_to<'a>(
    done: &'a [LaunchDone],
    app: &AppId,
    asked_at: Stamp,
) -> Option<&'a LaunchDone> {
    done.iter()
        .find(|entry| entry.app() == app && entry.requested_at() >= asked_at)
}

/// The timing line, `launch desktop_id=<id> unit=<unit> requested_at=<ns> spawned_at=<ns>
/// pid=<pid>`: fields are `key=value` with no spaces in a value (whitespace becomes `_`); no unit
/// reads `-` and no process reads `0`.
pub fn launch_line(timing: &LaunchTiming) -> String {
    let unit = timing.unit.as_ref().map_or("-", |unit| unit.0.as_str());
    let pid = timing.pid.map_or(0, |pid| pid.0);
    format!(
        "launch desktop_id={} unit={} requested_at={} spawned_at={} pid={pid}",
        one_word(&timing.app.0),
        one_word(unit),
        timing.requested_at.0,
        timing.spawned_at.0,
    )
}

/// `text` with every whitespace character replaced by `_`.
fn one_word(text: &str) -> String {
    text.chars()
        .map(|c| if c.is_whitespace() { '_' } else { c })
        .collect()
}
