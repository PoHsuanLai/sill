//! The scopes we started: the dock stops a launch bounce when the scope
//! exits (design/10 §10.3.5), and Force Quit needs the unit.

use serde::{Deserialize, Serialize};
use sill_launcher::AppId;

use super::LaunchDone;

/// A transient systemd scope's unit name: `app-sill-<id>-<nonce>.scope`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ScopeUnit(pub String);

/// Every scope we started that has not exited, oldest first, and how the last few launch requests
/// ended (so a caller that asked can read its answer).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Launches {
    pub scopes: Vec<LaunchedScope>,
    /// The last [`DONE_KEPT`] requests' ends, oldest first.
    pub done: Vec<LaunchDone>,
}

/// How many request ends [`Launches::done`] keeps.
pub const DONE_KEPT: usize = 8;

/// One running scope.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LaunchedScope {
    pub app: AppId,
    pub unit: ScopeUnit,
}
