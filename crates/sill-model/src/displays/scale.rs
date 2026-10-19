//! An output's scale as the compositor reports it. sill never decides one: the compositor picks
//! a display's scale when it first sees the display, and sill only shows what it chose.

use serde::{Deserialize, Serialize};

/// An output scale in hundredths (`150` is 1.5).
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default, Serialize, Deserialize,
)]
#[serde(transparent)]
pub struct DisplayScale(pub u16);

impl DisplayScale {
    pub const ONE: DisplayScale = DisplayScale(100);
}
