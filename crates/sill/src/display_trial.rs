//! The daemon's half of the display trial: the compositor's notes open, restart and close the
//! question on the link ([`apply`], called from [`crate::daemon::compositor::apply`]), the trial
//! surface on the sheets' output shows it, and the answer comes back on the link.
//! [`Outgoing`] sends each answer to the host as a command for the desktop
//! arm. The compositor reverts by itself when the time runs out, so nothing here keeps time.

use sill_model::desktop::{DesktopCommand, ShellNote};
use sill_ui_kit::links::{DisplayTrialLink, TrialAnswered, TrialReply};
use tokio::sync::watch;

use crate::outgoing::Outgoing;

/// What a note of the display trial does to the question on `link`. Any other note changes
/// nothing.
///
/// ```
/// use sill::display_trial::apply;
/// use sill_model::desktop::{DisplayTrial, ShellNote, TrialId};
/// use sill_ui_kit::links::{DisplayTrialLink, SheetLink};
///
/// let link = DisplayTrialLink::new(SheetLink::new());
/// let trial = DisplayTrial { trial: TrialId::new(1), seconds: 15 };
/// apply(&link, ShellNote::DisplaysTrial(trial));
/// assert_eq!(link.latest().map(|run| run.trial), Some(trial));
/// apply(&link, ShellNote::DisplaysTrialEnd(TrialId::new(1)));
/// assert_eq!(link.latest(), None);
/// ```
pub fn apply(link: &DisplayTrialLink, note: ShellNote) {
    match note {
        ShellNote::DisplaysTrial(trial) => link.begin(trial),
        ShellNote::DisplaysTrialEnd(trial) => link.end(trial),
        _ => {}
    }
}

/// The command an answer is.
pub fn command_of(answered: &TrialAnswered) -> DesktopCommand {
    DesktopCommand::AnswerTrial {
        trial: answered.trial,
        answer: answered.answer,
    }
}

impl Outgoing for DisplayTrialLink {
    type Item = TrialReply;

    fn follow(&self) -> watch::Receiver<Option<TrialAnswered>> {
        self.subscribe_answers()
    }

    fn commands(answered: &TrialAnswered) -> Vec<DesktopCommand> {
        vec![command_of(answered)]
    }
}

#[cfg(test)]
mod tests {
    use sill_model::desktop::{DesktopCommand, DisplayTrial, ShellNote, TrialAnswer, TrialId};
    use sill_ui_kit::links::{DisplayTrialLink, SheetLink};

    use super::{apply, command_of};

    fn trial(id: u32) -> DisplayTrial {
        DisplayTrial {
            trial: TrialId::new(id),
            seconds: 15,
        }
    }

    #[test]
    fn the_notes_open_restart_and_close_the_question() {
        let link = DisplayTrialLink::new(SheetLink::new());
        apply(&link, ShellNote::DisplaysTrial(trial(1)));
        let first = link.latest().expect("open");
        apply(&link, ShellNote::DisplaysTrial(trial(1)));
        let again = link.latest().expect("open");
        assert_ne!(
            first.serial, again.serial,
            "a restart is told from the first send"
        );
        apply(&link, ShellNote::DisplaysTrialEnd(TrialId::new(2)));
        assert!(link.latest().is_some(), "another trial's end leaves it");
        apply(&link, ShellNote::DisplaysTrialEnd(TrialId::new(1)));
        assert_eq!(link.latest(), None);
    }

    #[test]
    fn a_note_of_another_kind_leaves_the_question_alone() {
        let link = DisplayTrialLink::new(SheetLink::new());
        apply(&link, ShellNote::Killed);
        assert_eq!(link.latest(), None);
    }

    #[test]
    fn an_answer_is_the_trial_with_what_was_said() {
        let link = DisplayTrialLink::new(SheetLink::new());
        link.answer(TrialId::new(7), TrialAnswer::Keep);
        let answered = link.last_answer().expect("answered");
        assert_eq!(
            command_of(&answered),
            DesktopCommand::AnswerTrial {
                trial: TrialId::new(7),
                answer: TrialAnswer::Keep,
            }
        );
    }
}
