//! What the badges service hears and what it can be told.

use serde::{Deserialize, Serialize};
use sill_launcher::AppId;

use crate::units::{Permille, Urgency, Visibility};

/// One LauncherEntry property.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum BadgeProp {
    /// `count` (x); negative clamps to 0.
    Count(i64),
    /// `count-visible` (b).
    CountVisibility(Visibility),
    /// `progress` (d) as thousandths, clamped to 0..=1000.
    Progress(Permille),
    /// `progress-visible` (b).
    ProgressVisibility(Visibility),
    /// `urgent` (b).
    Urgency(Urgency),
}

/// One change on the bus.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum BadgeEvent {
    /// An `Update` signal for `app`, its properties in signal order.
    Update { app: AppId, props: Vec<BadgeProp> },
    /// The app's sender left the bus: nothing it said stands.
    Cleared(AppId),
    /// Forget everything.
    ClearedAll,
}

/// What the badges service can be told.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BadgesCommand {
    /// Forget every badge (after `sill reload`).
    Clear,
}
