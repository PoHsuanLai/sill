//! How long a permission question takes no answer after it appears.

use std::time::Duration;

/// How long the sheet answers nothing after it mounts: a keypress or click already in flight for
/// the window behind it must not allow a program a power. Every question gets it, the portal's and
/// the compositor's alike, and each question in the line starts its own wait when it is drawn.
/// The compositor may refuse an answer that comes sooner than 500 ms after its question, so this
/// stays above that.
pub const PERMISSION_ARM_AFTER: Duration = Duration::from_millis(600);
