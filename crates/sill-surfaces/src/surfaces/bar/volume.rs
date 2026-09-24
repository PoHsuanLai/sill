//! The volume status item and its menu (a quire `Slider`, output device, mute).

use dioxus::prelude::*;
use shell_host::OutputInfo;

use super::menus::BarMenu;
use super::status::{volume_heading, volume_icon};
use super::status_item::StatusIcon;
use crate::hooks::use_volume;

/// The volume status item and its menu (a quire `Slider`, output device, mute).
#[component]
pub fn VolumeItem(output: OutputInfo) -> Element {
    // Every bar item is told its output (the frozen signature); this one reads the same on all.
    let _ = &output;
    let volume = use_volume()();
    let tooltip = format!("{}: {}%", volume_heading(&volume), volume.level.0);
    rsx! {
        StatusIcon { menu: BarMenu::Volume, icon: volume_icon(&volume), tooltip }
    }
}
