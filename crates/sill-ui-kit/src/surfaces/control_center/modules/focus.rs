//! The Focus tile: Do Not Disturb as a quire `ModuleTile` (the `Moon` glyph),
//! [`crate::surfaces::control_center::focus_status`] as its line. Reads `use_focus_mode`; sets
//! through the root context's `sill_settings::FocusLink`.

use dioxus::prelude::*;
use ds::{DiscMotion, Icon, ModuleState, ModuleTile, TextLine};
use sill_settings::{FocusLink, FocusMode};

use super::cc_module::{CcCx, CcModule, Span};
use crate::hooks::use_focus_mode;
use crate::surfaces::control_center::focus_status;
use crate::surfaces::control_center::modules::flipped;

/// The Focus tile.
#[component]
pub fn FocusModule() -> Element {
    let mode = use_focus_mode()();
    let link = use_context::<FocusLink>();
    rsx! {
        ModuleTile {
            glyph: Icon::Moon,
            // On grows the filled moon in, springing under the tile's own press (design/26 G23).
            disc: DiscMotion::Morph(Icon::MoonFilled),
            title: "Focus",
            status: Some(TextLine::from(focus_status(mode))),
            state: focus_state(mode),
            onclick: move |_| {
                // Gone: the daemon's focus task has stopped; the tile keeps what it last read.
                let _delivery = link.set(mode_of(flipped(focus_state(mode))));
            },
        }
    }
}

pub(crate) fn focus_state(mode: FocusMode) -> ModuleState {
    match mode {
        FocusMode::DoNotDisturb => ModuleState::On,
        FocusMode::Off => ModuleState::Off,
    }
}

pub(crate) fn mode_of(state: ModuleState) -> FocusMode {
    match state {
        ModuleState::On | ModuleState::Busy => FocusMode::DoNotDisturb,
        ModuleState::Off => FocusMode::Off,
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
    use super::{focus_state, mode_of};
    use sill_settings::FocusMode;

    #[test]
    fn the_switch_is_do_not_disturb() {
        for mode in [FocusMode::Off, FocusMode::DoNotDisturb] {
            assert_eq!(mode_of(focus_state(mode)), mode, "{mode:?}");
        }
    }
}
