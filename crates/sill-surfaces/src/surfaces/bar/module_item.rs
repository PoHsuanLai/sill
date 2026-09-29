//! A control center module as its own bar item (`control_center.menu_bar_*`): a status icon
//! whose dropdown is the module's own component (`PopupBody::Module`).

use dioxus::prelude::*;
use ds::{Icon, StatusState};
use shell_host::OutputInfo;
use sill_settings::ControlCenterModule;

use super::glyph_state::{battery_glyph, bluetooth_glyph_state, network_glyph, volume_glyph};
use super::low_battery::use_low_cue;
use super::menus::BarMenu;
use super::status::{battery_lines, network_lines, volume_heading};
use super::status_item::StatusIcon;
use crate::hooks::{
    use_battery, use_bluetooth, use_network, use_now_playing, use_settings, use_volume, use_wifi,
};
use crate::surfaces::control_center::bluetooth_status;

/// `module`'s bar item on `output` (its glyph is the module's status glyph: quire's Wi-Fi,
/// battery, volume and Bluetooth glyphs through `glyph_state`).
#[component]
pub fn ModuleItem(output: OutputInfo, module: ControlCenterModule) -> Element {
    // Every bar item is told its output (the frozen signature); these read the same on all.
    let _ = &output;
    match module {
        ControlCenterModule::Sound => rsx! { SoundItem {} },
        ControlCenterModule::Wifi => rsx! { WifiItem {} },
        ControlCenterModule::Battery => rsx! { BatteryItem {} },
        ControlCenterModule::Bluetooth => rsx! { BluetoothItem {} },
        ControlCenterModule::Display => rsx! {
            StatusIcon {
                menu: BarMenu::Display,
                icon: Icon::Sun,
                tooltip: "Display".to_owned(),
            }
        },
        ControlCenterModule::NowPlaying => rsx! { NowPlayingItem {} },
        // Not bar items (`in_menu_bar` hides them); drawn as nothing if named anyway.
        ControlCenterModule::Focus
        | ControlCenterModule::KeyboardBrightness
        | ControlCenterModule::Appearance => rsx! {},
    }
}

/// Sound: the volume glyph (quire's `VolumeGlyph` state, the Sound module's source too).
#[component]
fn SoundItem() -> Element {
    let volume = use_volume()();
    let tooltip = format!("{}: {}%", volume_heading(&volume), volume.level.0);
    rsx! {
        StatusIcon {
            menu: BarMenu::Volume,
            icon: StatusState::Volume(volume_glyph(&volume)),
            tooltip,
        }
    }
}

/// Wi-Fi: the Wi-Fi glyph (searching while it joins, its bars and badge once joined, the slash
/// with the radio off), or the wire's.
#[component]
fn WifiItem() -> Element {
    let network = use_network()();
    let radio = use_wifi()().radio;
    let [link, _] = network_lines(&network);
    rsx! {
        StatusIcon { menu: BarMenu::Network, icon: network_glyph(&network, radio), tooltip: link }
    }
}

/// Battery: the battery glyph by level and power, nudged once per low crossing; hidden
/// without a battery (UPower's display device absent: a desktop).
#[component]
fn BatteryItem() -> Element {
    let battery = use_battery()();
    let low = use_settings()().bar.battery_low_percent;
    let state = battery_glyph(&battery, low);
    let cue = use_low_cue(state);
    let Some(state) = state else {
        return rsx! {};
    };
    let [level, _] = battery_lines(&battery);
    rsx! {
        StatusIcon {
            menu: BarMenu::Battery,
            icon: StatusState::Battery(state),
            tooltip: level,
            nudge: Some(cue),
        }
    }
}

/// Bluetooth: off, on, connecting, connected, or a failed connect.
#[component]
fn BluetoothItem() -> Element {
    let bluetooth = use_bluetooth()();
    rsx! {
        StatusIcon {
            menu: BarMenu::Bluetooth,
            icon: StatusState::Bluetooth(bluetooth_glyph_state(&bluetooth)),
            tooltip: format!("Bluetooth: {}", bluetooth_status(&bluetooth)),
        }
    }
}

/// Now Playing: shown while any player is up.
#[component]
fn NowPlayingItem() -> Element {
    let media = use_now_playing()();
    if media.players.is_empty() {
        return rsx! {};
    }
    rsx! {
        StatusIcon { menu: BarMenu::NowPlaying, icon: Icon::Play, tooltip: "Now Playing".to_owned() }
    }
}
