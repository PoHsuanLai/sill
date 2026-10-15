//! The Sound module: a [`crate::control_center::LevelModule`] headed by the bar item's
//! volume glyph, the output device's name as its title, and the level, whose speaker draws the
//! same `VolumeState` (the module and the bar read one source). It reads no service:
//! the caller ([`super::Module`], the bar's volume dropdown) passes `volume_glyph(&volume)` and
//! `Volume::level` and sends `VolumeCommand::SetLevel` from `on_change`, so the same component
//! serves anywhere a level is shown. Choosing another output device needs a sink list and a
//! `SetSink` the volume service does not have; it is left for later.

use dioxus::prelude::*;
use ds::components::content::status::volume::VolumeState;
use ds::prelude::*;

use super::cc_module::{BarItemFace, CcCx, CcModule, Span};
use super::wired::WiredSound;
use crate::bar::{BarMenu, StatusIcon};
use crate::control_center::LevelModule;
use sill_services::VolumeService;
use sill_settings::{ControlCenterSettings, InMenuBar};
use sill_ui_kit::hooks::use_service;
use sill_ui_kit::status::glyph::volume_glyph;
use sill_ui_kit::status::lines::volume_heading;

/// The Sound module at `level` on `device` (`Volume::sink`), its glyphs drawing `volume`.
#[component]
pub fn SoundModule(
    volume: VolumeState,
    level: Fraction,
    device: String,
    on_change: EventHandler<Fraction>,
    #[props(default)] availability: Availability,
) -> Element {
    rsx! {
        LevelModule {
            level_glyph: volume,
            title: device,
            level,
            on_change,
            availability,
        }
    }
}

/// The Sound item in the bar: the volume glyph (quire's `VolumeGlyph` state, the Sound module's
/// source too).
#[component]
fn SoundItem() -> Element {
    let volume = use_service::<VolumeService>()();
    let tooltip = format!("{}: {}%", volume_heading(&volume), volume.level.0);
    rsx! {
        StatusIcon {
            menu: BarMenu::Volume,
            icon: StatusState::Volume(volume_glyph(&volume)),
            tooltip,
        }
    }
}

/// The Sound module as the control center and the bar draw it.
pub(crate) struct SoundCc;

impl CcModule for SoundCc {
    fn span(&self) -> Span {
        Span::Row
    }

    fn tile(&self, _cx: &CcCx) -> Element {
        rsx! { WiredSound {} }
    }

    fn menu_bar(&self, settings: &ControlCenterSettings) -> InMenuBar {
        settings.menu_bar_sound
    }

    fn bar_item(&self) -> Option<BarItemFace> {
        Some(BarItemFace {
            menu: BarMenu::Volume,
            item: || rsx! { SoundItem {} },
        })
    }
}
