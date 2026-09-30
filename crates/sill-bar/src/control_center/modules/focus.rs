//! The Focus tile: Do Not Disturb as a quire `ModuleTile` (the `Moon` glyph),
//! [`crate::control_center::focus_status`] as its line. Reads `use_focus_mode`; sets
//! through the root context's `sill_ui_kit::links::FocusLink`.

use dioxus::prelude::*;
use ds::prelude::*;
use ds_shell::prelude::*;
use sill_settings::FocusMode;
use sill_ui_kit::links::FocusLink;

use super::cc_module::{CcCx, CcModule, Span};
use crate::control_center::focus_status;
use crate::control_center::modules::flipped;
use sill_ui_kit::hooks::use_focus_mode;

/// The Focus tile.
#[component]
pub fn FocusModule() -> Element {
    let mode = use_focus_mode()();
    let link = use_context::<FocusLink>();
    rsx! {
        ModuleTile {
            glyph: Icon::Moon,
            title: "Focus",
            status: Some(TextLine::from(focus_status(mode))),
            value: focus_state(mode),
            onclick: move |_| {
                // Gone: the daemon's focus task has stopped; the tile keeps what it last read.
                let _delivery = link.set(mode_of(flipped(focus_state(mode))));
            },
        }
    }
}

pub(crate) fn focus_state(mode: FocusMode) -> Check {
    match mode {
        FocusMode::DoNotDisturb => Check::On,
        FocusMode::Off => Check::Off,
    }
}

pub(crate) fn mode_of(state: Check) -> FocusMode {
    match state {
        Check::On | Check::Mixed => FocusMode::DoNotDisturb,
        Check::Off => FocusMode::Off,
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
