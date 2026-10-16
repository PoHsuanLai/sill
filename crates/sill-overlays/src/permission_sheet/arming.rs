//! How long a permission question takes no answer after it appears.

use std::time::Duration;

use crate::arming::QUESTION_ARM_AFTER;

/// How long the sheet answers nothing after it mounts: every question gets it, the portal's and
/// the compositor's alike, and each question in the line starts its own wait when it is drawn.
/// It is the wait every question of the overlays takes ([`QUESTION_ARM_AFTER`]).
pub const PERMISSION_ARM_AFTER: Duration = QUESTION_ARM_AFTER;
