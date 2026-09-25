//! A control center module as its own bar item (`control_center.menu_bar_*`): a status icon
//! whose dropdown is the module's own component (`PopupBody::Module`). Replaces the pre-M5
//! volume, network and battery items, whose menus become the Sound, Wi-Fi and Battery modules.

use dioxus::prelude::*;
use ds::Icon;
use shell_host::OutputInfo;
use sill_services::Presence;
use sill_settings::ControlCenterModule;

use super::menus::BarMenu;
use super::status::{
    battery_icon, battery_lines, network_icon, network_lines, volume_heading, volume_icon,
};
use super::status_item::StatusIcon;
use crate::hooks::{use_battery, use_bluetooth, use_network, use_now_playing, use_volume};
use crate::surfaces::control_center::bluetooth_status;
use crate::surfaces::control_center::modules::bluetooth::bluetooth_glyph;

/// `module`'s bar item on `output` (its glyph follows the module's state: signal strength,
/// volume level, battery level).
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
        ControlCenterModule::Focus | ControlCenterModule::Appearance => rsx! {},
    }
}

/// Sound: the volume glyph by level and mute.
#[component]
fn SoundItem() -> Element {
    let volume = use_volume()();
    let tooltip = format!("{}: {}%", volume_heading(&volume), volume.level.0);
    rsx! {
        StatusIcon { menu: BarMenu::Volume, icon: volume_icon(&volume), tooltip }
    }
}

/// Wi-Fi: the network glyph (Wi-Fi strength, or the wire).
#[component]
fn WifiItem() -> Element {
    let network = use_network()();
    let [link, _] = network_lines(&network);
    rsx! {
        StatusIcon { menu: BarMenu::Network, icon: network_icon(&network), tooltip: link }
    }
}

/// Battery: the level glyph; hidden without a battery (UPower's display device absent: a
/// desktop).
#[component]
fn BatteryItem() -> Element {
    let battery = use_battery()();
    if battery.presence == Presence::Absent {
        return rsx! {};
    }
    let [level, _] = battery_lines(&battery);
    rsx! {
        StatusIcon { menu: BarMenu::Battery, icon: battery_icon(&battery), tooltip: level }
    }
}

/// Bluetooth: off, on, or connected.
#[component]
fn BluetoothItem() -> Element {
    let bluetooth = use_bluetooth()();
    rsx! {
        StatusIcon {
            menu: BarMenu::Bluetooth,
            icon: bluetooth_glyph(&bluetooth),
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
