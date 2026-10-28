//! The Focus tile: Do Not Disturb as a quire `ModuleTile` (the `Moon` glyph),
//! [`crate::control_center::focus_status`] as its line. Reads `use_focus_mode`; a click asks for
//! the shell's `ToggleFocus` action, which writes `settings.toml`.

use dioxus::prelude::*;
use ds::prelude::*;
use sill_launcher::actions::ActionId;
use sill_settings::OnOff;
use sill_shell_kit::prelude::*;

use super::cc_module::{CcCx, CcModule, Span};
use crate::control_center::focus_status;
use sill_ui_kit::hooks::{use_entrance, use_focus_mode};

/// The Focus tile.
#[component]
pub fn FocusModule() -> Element {
    let mode = use_focus_mode()();
    let entrance = use_entrance();
    rsx! {
        ModuleTile {
            glyph: Icon::Moon,
            title: "Focus",
            status: Some(TextLine::from(focus_status(mode))),
            value: focus_state(mode),
            onclick: move |_| entrance.do_it(ActionId::ToggleFocus),
        }
    }
}

pub(crate) fn focus_state(mode: OnOff) -> Check {
    match mode {
        OnOff::On => Check::On,
        OnOff::Off => Check::Off,
    }
}

/// The Focus module as the control center draws it: a tile, never a bar item.
pub(crate) struct FocusCc;

impl CcModule for FocusCc {
    fn span(&self) -> Span {
        Span::One
    }

    fn tile(&self, _cx: &CcCx) -> Element {
        rsx! { FocusModule {} }
    }
}

#[cfg(test)]
mod tests {
    use super::{Check, focus_state};
    use sill_settings::OnOff;

    #[test]
    fn the_switch_is_do_not_disturb() {
        assert_eq!(focus_state(OnOff::On), Check::On);
        assert_eq!(focus_state(OnOff::Off), Check::Off);
    }
}
