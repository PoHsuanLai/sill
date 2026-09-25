//! One file per control center module (design/13 §13.3.7's list), each a component over its
//! hook, drawn only with quire components (`Toggle`, `Slider`, `SegmentedControl`,
//! `AppearancePicker`, `SectionHeader`, `Button{Mini}`, `IconButton`, `Glyph`, `IconView`), and
//! [`Module`], which picks one by `control_center.modules`' word.

pub mod appearance;
pub mod battery;
pub mod bluetooth;
pub mod display;
pub mod focus;
pub mod now_playing;
pub mod power_profile;
pub mod sound;
pub mod wifi;

pub use appearance::AppearanceModule;
pub use battery::BatteryModule;
pub use bluetooth::BluetoothModule;
pub use display::DisplayModule;
pub use focus::FocusModule;
pub use now_playing::NowPlayingModule;
pub use power_profile::PowerProfileControl;
pub use sound::SoundModule;
pub use wifi::WifiModule;

use dioxus::prelude::*;
use sill_services::OutputName;
use sill_settings::ControlCenterModule;

use super::page::Detail;

/// The module `module` names. `output` is the bar's (Display's brightness target); `on_detail`
/// is a chevron's (Wi-Fi's and Bluetooth's).
#[component]
pub fn Module(
    module: ControlCenterModule,
    output: Option<OutputName>,
    on_detail: EventHandler<Detail>,
) -> Element {
    let _ = (module, output, on_detail);
    todo!("M5 fill: control-center (c)")
}
