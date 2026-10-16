//! The display trial: a program other than the shell changed the displays and the compositor asks
//! the person whether to keep the new setup (`quire_shell_v1` version 6, `displays_trial`). Pure:
//! the question's words and its countdown arithmetic. The compositor reverts by itself when the
//! time runs out, so the countdown only tells the person how long is left; it never decides.

use casement_protocols::vocab::TrialId;

/// The question's headline, as the Mac asks it.
pub const TRIAL_TITLE: &str = "Keep these display settings?";

/// The label of the button that goes back to the previous setup; it is the default, so a stray
/// Return can never leave a screen the person cannot see.
pub const REVERT_LABEL: &str = "Revert";

/// The label of the button that keeps the new setup.
pub const KEEP_LABEL: &str = "Keep Changes";

/// One trial, as the compositor opens or restarts it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DisplayTrial {
    /// The trial's number: the answer and the end name it. A restart may carry the same number.
    pub trial: TrialId,
    /// How long the person has, in seconds, counted from this event.
    pub seconds: u32,
}

/// Whole seconds left of `seconds` once `elapsed_ms` milliseconds have passed.
pub fn seconds_left(seconds: u32, elapsed_ms: u64) -> u32 {
    let gone = u32::try_from(elapsed_ms / 1000).unwrap_or(u32::MAX);
    seconds.saturating_sub(gone)
}

/// Milliseconds from `elapsed_ms` to the next whole second of the countdown, so the line changes
/// on the second rather than drifting.
pub fn until_next_second(elapsed_ms: u64) -> u64 {
    1000 - elapsed_ms % 1000
}

/// The line under the headline for `left` seconds to go.
pub fn countdown_line(left: u32) -> String {
    match left {
        0 => "Reverting now".to_owned(),
        1 => "Reverting in 1 second".to_owned(),
        n => format!("Reverting in {n} seconds"),
    }
}

#[cfg(test)]
mod tests {
    use super::{countdown_line, seconds_left, until_next_second};

    #[test]
    fn the_seconds_left_count_down_on_the_second_and_stop_at_zero() {
        let cases = [
            (15, 0, 15),
            (15, 999, 15),
            (15, 1000, 14),
            (15, 14_500, 1),
            (15, 15_000, 0),
            (15, 99_000, 0),
            (15, u64::MAX, 0),
        ];
        for (seconds, elapsed, want) in cases {
            assert_eq!(seconds_left(seconds, elapsed), want, "{seconds} {elapsed}");
        }
    }

    #[test]
    fn the_wait_runs_to_the_next_whole_second() {
        let cases = [(0, 1000), (1, 999), (999, 1), (1000, 1000), (2500, 500)];
        for (elapsed, want) in cases {
            assert_eq!(until_next_second(elapsed), want, "{elapsed}");
        }
    }

    #[test]
    fn the_line_says_how_long_is_left() {
        let cases = [
            (15, "Reverting in 15 seconds"),
            (2, "Reverting in 2 seconds"),
            (1, "Reverting in 1 second"),
            (0, "Reverting now"),
        ];
        for (left, want) in cases {
            assert_eq!(countdown_line(left), want, "{left}");
        }
    }
}
