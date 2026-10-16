//! How long a question that comes from outside the shell takes no answer after it appears.

use std::time::Duration;

/// How long a compositor's or app's question answers nothing after it is drawn: a keypress or
/// click already in flight for the window behind it must not answer for the person. The
/// permission sheet and the display trial take it alike. The compositor may refuse an answer
/// that comes sooner than 500 ms after its question, so this stays above that.
pub const QUESTION_ARM_AFTER: Duration = Duration::from_millis(600);
