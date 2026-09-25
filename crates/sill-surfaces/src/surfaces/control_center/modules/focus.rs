//! The Focus tile: Do Not Disturb as a quire `Toggle` (the `Moon` glyph),
//! [`crate::surfaces::control_center::focus_status`] as its line. Reads `use_focus_mode`; sets
//! through the root context's `sill_settings::FocusLink`.

use dioxus::prelude::*;
use ds::Icon;
use sill_settings::{FocusLink, FocusMode};

use crate::hooks::use_focus_mode;
use crate::surfaces::control_center::focus_status;
use crate::surfaces::control_center::parts::{ModuleState, ModuleTile};

/// The Focus tile.
#[component]
pub fn FocusModule() -> Element {
    let mode = use_focus_mode()();
    let link = use_context::<FocusLink>();
    rsx! {
        ModuleTile {
            glyph: Icon::Moon,
            title: "Focus".to_owned(),
            status: focus_status(mode).to_owned(),
            state: focus_state(mode),
            on_toggle: move |to: ModuleState| {
                // Gone: the daemon's focus task has stopped; the tile keeps what it last read.
                let _delivery = link.set(mode_of(to));
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
        ModuleState::On => FocusMode::DoNotDisturb,
        ModuleState::Off => FocusMode::Off,
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
