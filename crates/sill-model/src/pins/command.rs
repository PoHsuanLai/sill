//! What the pins service hears and what it can be told.

use serde::{Deserialize, Serialize};
use sill_launcher::AppId;

use super::DockPins;

/// Where a new pin goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum PinAt {
    /// After the last pin (`sill dock pin`, Keep in Dock).
    End,
    /// At this slot (a drop between tiles), clamped to the end.
    Index(u32),
}

/// One change to the pins.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum PinEvent {
    /// Pin `app` at `at`; an app already pinned moves there.
    Pin { app: AppId, at: PinAt },
    /// Unpin `app`; nothing if it is not pinned.
    Unpin(AppId),
    /// Move a pinned `app` to slot `to` (drag reorder), clamped; nothing if not pinned.
    Move { app: AppId, to: u32 },
    /// The file changed on disk.
    Replaced(DockPins),
}
