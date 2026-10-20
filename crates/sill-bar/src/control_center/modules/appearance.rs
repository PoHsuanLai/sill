//! The Appearance module: the theme as a quire `SegmentedControl`, its value from
//! `use_appearance`, a change written by `write_appearance` (the file the daemon's
//! appearance watch then publishes to every surface). The accent is not here: it lives
//! in Settings (cleanup DECISIONS: the control center keeps the dark-mode switch only).

use dioxus::prelude::*;
use ds::components::controls::segmented::Tracking;
use ds::prelude::*;
use sill_shell_kit::prelude::*;

use super::cc_module::{CcCx, CcModule, Span};
use crate::control_center::modules::use_plate;
use sill_ui_kit::appearance_write::write_appearance;
use sill_ui_kit::hooks::use_appearance;

/// The Appearance module.
#[component]
pub fn AppearanceModule() -> Element {
    let environment = use_appearance()();
    let value = environment.settings.appearance.appearance();
    let system = environment.system.scheme;
    let hint = match (value.theme, system) {
        (Theme::System, scheme) => Some(scheme.label().to_owned()),
        (Theme::Light | Theme::Dark, _) => None,
    };
    rsx! {
        ModulePanel { plate: use_plate(),
            div { class: "sill-cc-appearance",
                SectionHeader { title: "Theme", value: hint }
                SegmentedControl::<Theme> {
                    label: "Theme",
                    choices: Choice::pairs(Theme::ALL.iter().map(|theme| (*theme, theme.label()))),
                    tracking: Tracking::SelectOne(value.theme),
                    onchange: move |theme| write_appearance(Appearance { theme, ..value }),
                }
            }
        }
    }
}

/// The Appearance module as the control center draws it: a row, never a bar item.
pub(crate) struct AppearanceCc;

impl CcModule for AppearanceCc {
    fn span(&self) -> Span {
        Span::Row
    }

    fn tile(&self, _cx: &CcCx) -> Element {
        rsx! { AppearanceModule {} }
    }
}
