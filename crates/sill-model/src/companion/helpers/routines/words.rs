//! The words the routines wear. Plain ones: a routine is something a helper does by itself at a
//! time; nothing here names how the machinery works.

use companion_wire::{RoutineLine, RoutineShown, RunShown};

use super::draft::Repeat;

/// Said under the schedule wherever a routine's time is shown.
pub const MISSED_SKIPPED: &str = "If your computer is off at that time, the run is skipped.";

/// Said when a helper has no routine.
pub const NO_ROUTINES: &str = "Nothing runs by itself yet.";

/// The question before a routine goes.
pub fn remove_routine_question(goal: &str) -> String {
    format!("Remove \u{201c}{goal}\u{201d}? Runs it already did stay in your history.")
}

/// How often, as a choice reads.
pub fn repeat_label(repeat: Repeat) -> &'static str {
    match repeat {
        Repeat::Daily => "Every day",
        Repeat::Weekdays => "Weekdays",
        Repeat::EveryFewHours => "Every few hours",
    }
}

/// Whether a routine runs, in a few words.
pub fn state_label(state: RoutineShown) -> &'static str {
    match state {
        RoutineShown::On => "On",
        RoutineShown::Paused => "Paused",
        RoutineShown::Stopped => "Stopped: three runs in a row did not finish",
    }
}

/// How the last run came out, when there was one.
pub fn last_run_line(line: &RoutineLine) -> Option<String> {
    let run = line.last.as_ref()?;
    let state = match run.state {
        RunShown::Working => "Working on it",
        RunShown::Done => "Done",
        RunShown::DidNotFinish => "Did not finish",
        RunShown::Skipped => "Skipped",
    };
    Some(match &run.note {
        Some(note) => format!("Last run: {state}. {note}"),
        None => format!("Last run: {state}"),
    })
}
