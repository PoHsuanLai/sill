//! The display trial: when a program other than the shell changes the displays, the compositor
//! applies the change and asks whether to keep it, going back by itself if the person does not say
//! so in time (a bad mode can blank the screen). The question is the Mac's: quire's `Alert`
//! "Keep these display settings?" centred on the sheets' output over an Overlay layer that takes
//! the keyboard, with a line under the headline that counts the seconds down ("Reverting in 15
//! seconds", changed every second). Revert comes first, so it is the default (Return) and the
//! button Escape presses; Keep Changes keeps the new setup.
//!
//! The trial arrives on [`sill_ui_kit::links::DisplayTrialLink`] (`ShellNote::DisplaysTrial`); a
//! further change while it is open arrives again, with the same number or a new one, and the
//! sheet starts over with the new time. The compositor's end of the trial takes the sheet down
//! without an answer. Nothing here reverts: the compositor does when the time runs out.
//!
//! No trial takes an answer for [`QUESTION_ARM_AFTER`](crate::arming::QUESTION_ARM_AFTER) after
//! it is drawn, the permission sheet's wait: the buttons are disabled and Return, Escape and
//! clicks do nothing until then (quire's `Alert` `arm_after`).

mod io;
mod surface;
mod view;

pub use surface::DisplayTrialSheet;
