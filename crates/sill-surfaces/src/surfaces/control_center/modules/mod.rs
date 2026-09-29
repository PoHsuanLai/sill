//! One file per control center module (design/13 §13.3.7's list), each a component over its
//! hook, drawn only with quire components (`Toggle`, `Slider`, `SegmentedControl`,
//! `AppearancePicker`, `SectionHeader`, `Button{Mini}`, `IconButton`, `Glyph`, `IconView`), and
//! [`Module`], which picks one by `control_center.modules`' word.

pub mod appearance;
pub mod battery;
pub mod bluetooth;
pub(crate) mod bluetooth_ask;
pub mod display;
pub mod focus;
pub mod keyboard;
pub mod media_face;
pub mod now_playing;
pub mod power_profile;
pub mod sound;
pub mod wifi;

mod wired;

pub use appearance::AppearanceModule;
pub use battery::BatteryModule;
pub use bluetooth::BluetoothModule;
pub use display::DisplayModule;
pub use focus::FocusModule;
pub use keyboard::KeyboardModule;
pub use now_playing::NowPlayingModule;
pub use power_profile::PowerProfileControl;
pub use sound::SoundModule;
pub use wifi::WifiModule;

use dioxus::prelude::*;
use sill_model::ids::OutputName;
use sill_settings::ControlCenterModule;

use ds::{ModuleState, PanelPlate};

use super::page::Detail;
use wired::{WiredDisplay, WiredSound};

/// The module `module` names, wired to its service: the Sound and Display modules take a level and
/// an `on_change`, which this reads from and sends to their services
/// (`use_service::<VolumeService>`, `use_service::<DisplaysService>`); the others read their own
/// hooks. `output` is the bar's (Display's brightness target); `on_detail` is a chevron's (Wi-Fi's
/// and Bluetooth's).
#[component]
pub fn Module(
    module: ControlCenterModule,
    output: Option<OutputName>,
    on_detail: EventHandler<Detail>,
) -> Element {
    match module {
        ControlCenterModule::Wifi => rsx! { WifiModule { on_detail } },
        ControlCenterModule::Bluetooth => rsx! { BluetoothModule { on_detail } },
        ControlCenterModule::Focus => rsx! { FocusModule {} },
        ControlCenterModule::Display => rsx! { WiredDisplay { output } },
        ControlCenterModule::KeyboardBrightness => rsx! { KeyboardModule {} },
        ControlCenterModule::Sound => rsx! { WiredSound {} },
        ControlCenterModule::NowPlaying => rsx! { NowPlayingModule {} },
        ControlCenterModule::Appearance => rsx! { AppearanceModule {} },
        ControlCenterModule::Battery => rsx! { BatteryModule {} },
    }
}

/// The plate a full-width module's `ModulePanel` takes: the tile's in the control center,
/// `Bare` in a module's own dropdown, whose popover card already is its plate (the dropdown
/// provides it).
pub(crate) fn use_plate() -> PanelPlate {
    try_use_context::<PanelPlate>().unwrap_or_default()
}

/// The state a press on a tile asks for: on from off, off from on or on the way.
pub(crate) fn flipped(state: ModuleState) -> ModuleState {
    match state {
        ModuleState::Off => ModuleState::On,
        ModuleState::On | ModuleState::Busy => ModuleState::Off,
    }
}
