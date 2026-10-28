//! The permissions mode in the assistant's input box: Auto or Manual for the conversation being
//! typed into, quire's `SegmentedControl` at the field's trailing edge. It shows the mode at all
//! times; a switch made while a step runs says "From the next step" beside it, and when an
//! administrator holds every conversation in Manual the control is shown but unavailable and the
//! same place says why. The key is `sill.switch-mode` (⇧⌘M), run by the panel's key claim.

use dioxus::prelude::*;
use docket_permissions_core::Mode;
use ds::components::content::label::{LabelRole, LabelStyle};
use ds::components::controls::segmented::Tracking;
use ds::prelude::*;
use ds::style::tokens::control_size::ControlSize;
use sill_launcher::{ActionKeys, OwnAction};
use sill_model::companion::{ModeView, SWITCH_NAME, TIP_NAME, mode_label};

use crate::launcher::action_keys::action_shortcut;

/// The modes the control offers, in the order it draws them.
const MODES: [Mode; 2] = [Mode::Auto, Mode::Manual];

/// The switch for the conversation's mode, with its note or help beside it.
#[component]
pub(crate) fn ModeSwitch(view: ModeView, onswitch: EventHandler<Mode>) -> Element {
    let availability = match view.locked {
        false => Availability::Enabled,
        true => Availability::Disabled,
    };
    let shortcut = action_shortcut(&ActionKeys::own(OwnAction::SwitchMode));
    let line = view.note.or(view.help);
    rsx! {
        div { class: "sill-launcher-mode",
            if let Some(line) = line {
                Label {
                    text: line,
                    role: LabelRole::Tertiary,
                    style: LabelStyle::Caption,
                }
            }
            Tooltip { text: TIP_NAME, shortcut,
                SegmentedControl::<Mode> {
                    label: SWITCH_NAME,
                    choices: Choice::pairs(MODES.iter().map(|mode| (*mode, mode_label(*mode)))),
                    tracking: Tracking::SelectOne(view.mode),
                    size: ControlSize::Small,
                    availability,
                    onchange: move |mode| onswitch.call(mode),
                }
            }
        }
    }
}
