//! How long a question that comes from outside the shell takes no answer after it appears.

use std::time::Duration;

/// How long a compositor's or app's question answers nothing after it is drawn when
/// the arming time is unset: a keypress or click already in flight for the window
/// behind it must not answer for the person. The permission sheet and the display trial take it
/// alike, from the key (`sill_settings::confirm_arm`), which is held to at least 500 ms because
/// the compositor refuses an answer that comes sooner. This is that key's default.
pub const QUESTION_ARM_AFTER: Duration = Duration::from_millis(600);
