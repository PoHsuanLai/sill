//! The units and two-state words every service shares. Integers throughout: floats stay out
//! of state (CONVENTIONS §12), so every type here is `Eq`.

use serde::{Deserialize, Serialize};

/// A percentage. Not clamped: a sink's volume can exceed 100.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default, Serialize, Deserialize,
)]
#[serde(transparent)]
pub struct Percent(pub u16);

/// Thousandths of one: a progress, a zoom factor above 1 (`1706` is 1.706), a curve value.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default, Serialize, Deserialize,
)]
#[serde(transparent)]
pub struct Permille(pub u32);

impl Permille {
    pub const ZERO: Permille = Permille(0);
    pub const ONE: Permille = Permille(1000);
}

/// Hundredths of a logical pixel: the dock's magnified geometry, which the design specifies to
/// 0.1 px (design/10-BEHAVIOUR-dock.md §10.8.1) and CSS draws fractionally.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default, Serialize, Deserialize,
)]
#[serde(transparent)]
pub struct Cpx(pub i32);

impl Cpx {
    /// `px` whole logical pixels.
    pub const fn px(px: i32) -> Cpx {
        Cpx(px * 100)
    }
}

/// A monotonic instant in whole milliseconds since an arbitrary origin (the daemon's start).
/// Machines take it as an argument; only the component converts the frame clock into one.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default, Serialize, Deserialize,
)]
#[serde(transparent)]
pub struct Stamp(pub u64);

impl Stamp {
    /// Milliseconds from `earlier` to `self`, zero if `earlier` is later.
    pub fn since(self, earlier: Stamp) -> u64 {
        self.0.saturating_sub(earlier.0)
    }

    /// `self` plus `ms` milliseconds.
    pub fn after(self, ms: u64) -> Stamp {
        Stamp(self.0.saturating_add(ms))
    }
}

/// Whole seconds (a battery's time to empty).
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default, Serialize, Deserialize,
)]
#[serde(transparent)]
pub struct Seconds(pub u64);

/// Whole minutes, for display.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default, Serialize, Deserialize,
)]
#[serde(transparent)]
pub struct Minutes(pub u32);

/// Whether a device or a service answered at all.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Presence {
    #[default]
    Absent,
    Present,
}

/// Whether a value is meant to be shown (LauncherEntry's `count-visible`, a workspace's
/// hidden state).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Visibility {
    #[default]
    Hidden,
    Shown,
}

/// Whether a control can be used: a menu row that can be picked, a media button that works.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Availability {
    #[default]
    Enabled,
    Disabled,
}
