//! The daemon's half of the Split View picker: the compositor's notes open, fill and close the
//! offer on the link ([`apply`], called from [`crate::daemon::compositor::apply`]), the picker
//! surfaces show it, and the answer comes back on the link. [`spawn_answers`] follows the answers
//! and sends each to the host as a command for the desktop arm.

use shell_host::HostHandle;
use sill_model::desktop::{DesktopCommand, DesktopView, ShellNote, cards_of, output_of};
use sill_ui_kit::links::{Links, SplitAnswer, SplitAnswered, SplitPickerLink};
use tokio::runtime::Handle;

use crate::daemon::msg::ShellMsg;

/// What a note of the Split View choice does to the offer on `link`. `view` is the desktop as it
/// is now: it names the first window's output and the words of each window on offer. Any other
/// note changes nothing.
///
/// ```
/// use sill::split_picker::apply;
/// use sill_model::desktop::{DesktopView, ShellNote, SplitSide};
/// use sill_ui_kit::links::SplitPickerLink;
///
/// let link = SplitPickerLink::new();
/// let note = ShellNote::SplitChoose { first: "w1".to_owned(), side: SplitSide::Left };
/// apply(&link, &DesktopView::default(), note);
/// assert_eq!(link.latest().map(|offer| offer.first), Some("w1".to_owned()));
/// ```
pub fn apply(link: &SplitPickerLink, view: &DesktopView, note: ShellNote) {
    match note {
        ShellNote::SplitChoose { first, side } => {
            let output = output_of(view, &first);
            link.begin(first, side, output);
        }
        ShellNote::SplitCandidate(window) => cards_of(view, &[window])
            .into_iter()
            .for_each(|card| link.add(card)),
        ShellNote::SplitChoiceEnd(first) => link.end(&first),
        _ => {}
    }
}

/// The command an answer is: the window picked, or the choice turned down.
///
/// ```
/// use sill::split_picker::command_of;
/// use sill_model::desktop::DesktopCommand;
/// use sill_ui_kit::links::{SplitAnswer, SplitPickerLink};
///
/// let link = SplitPickerLink::new();
/// link.answer("w1", SplitAnswer::Picked("w2".to_owned()));
/// let answered = link.last_answer().expect("answered");
/// assert_eq!(command_of(&answered), DesktopCommand::PickSplit("w2".to_owned()));
/// ```
pub fn command_of(answered: &SplitAnswered) -> DesktopCommand {
    match &answered.answer {
        SplitAnswer::Picked(window) => DesktopCommand::PickSplit(window.clone()),
        SplitAnswer::Cancelled => DesktopCommand::CancelSplit,
    }
}

/// Follow the answers on `links` on `rt` and send each to the host, until the host is gone.
pub fn spawn_answers(rt: &Handle, links: &Links, host: HostHandle<ShellMsg>) {
    let mut answers = links.split_picker.subscribe_answers();
    rt.spawn(async move {
        loop {
            if answers.changed().await.is_err() {
                return;
            }
            let Some(answered) = answers.borrow_and_update().clone() else {
                continue;
            };
            if host.send(ShellMsg::Command(command_of(&answered))).is_err() {
                return;
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use sill_model::desktop::{DesktopCommand, DesktopView, ShellNote, SplitSide};
    use sill_ui_kit::links::{SplitAnswer, SplitPickerLink};

    use super::{apply, command_of};

    /// The words of a window on offer come from the desktop's view (`sill-model`'s `cards_of`
    /// tests); here the view is empty, so a window it does not know has no card.
    #[test]
    fn the_notes_open_and_close_the_offer_and_a_window_the_view_lost_has_no_card() {
        let view = DesktopView::default();
        let link = SplitPickerLink::new();
        let choose = ShellNote::SplitChoose {
            first: "w1".to_owned(),
            side: SplitSide::Right,
        };
        apply(&link, &view, choose);
        apply(&link, &view, ShellNote::SplitCandidate("w2".to_owned()));
        let offer = link.latest().expect("offered");
        assert_eq!(
            (offer.first.as_str(), offer.side, offer.output.as_str()),
            ("w1", SplitSide::Right, "")
        );
        assert!(offer.cards.is_empty());
        apply(&link, &view, ShellNote::SplitChoiceEnd("w1".to_owned()));
        assert_eq!(link.latest(), None);
    }

    #[test]
    fn a_note_of_another_kind_leaves_the_offer_alone() {
        let link = SplitPickerLink::new();
        apply(&link, &DesktopView::default(), ShellNote::Killed);
        assert_eq!(link.latest(), None);
    }

    #[test]
    fn an_answer_is_a_pick_or_a_cancel() {
        let link = SplitPickerLink::new();
        let cases = [
            (
                SplitAnswer::Picked("w2".to_owned()),
                DesktopCommand::PickSplit("w2".to_owned()),
            ),
            (SplitAnswer::Cancelled, DesktopCommand::CancelSplit),
        ];
        for (answer, want) in cases {
            link.answer("w1", answer);
            let answered = link.last_answer().expect("answered");
            assert_eq!(command_of(&answered), want);
        }
    }
}
