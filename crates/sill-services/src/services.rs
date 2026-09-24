//! The one value every surface gets through `Root::context`: every service's receiver and
//! control. Clone is cheap (channels are reference-counted).

#![allow(unused_variables, dead_code)] // Freeze stubs

use tokio::runtime::Handle;
use tokio::sync::watch;

use crate::apps::{AppIndex, AppsCommand};
use crate::badges::{Badges, BadgesCommand};
use crate::battery::{Battery, BatteryCommand};
use crate::clock::{ClockCommand, ClockText};
use crate::control::Control;
use crate::cosmic_wl::{Toplevels, WlCommand, Workspaces};
use crate::launch::{LaunchCommand, Launches};
use crate::network::{Network, NetworkCommand};
use crate::pins::{DockPins, PinEvent};
use crate::results::{Results, SearchCommand};
use crate::sound::SoundCommand;
use crate::tray::{Tray, TrayCommand};
use crate::units::Presence;
use crate::volume::{Volume, VolumeCommand};
use crate::wallpaper::{WallpaperCommand, Wallpapers};

/// Every service's state receiver and command control. Surfaces read receivers through
/// `sill-surfaces::hooks` (`use_watch`) and send commands through the controls; nothing here is
/// a Dioxus signal (review rule: never pass a Signal through `Root::context`).
#[derive(Debug, Clone)]
pub struct Services {
    pub clock: watch::Receiver<ClockText>,
    pub clock_control: Control<ClockCommand>,
    pub battery: watch::Receiver<Battery>,
    pub battery_control: Control<BatteryCommand>,
    pub network: watch::Receiver<Network>,
    pub network_control: Control<NetworkCommand>,
    pub volume: watch::Receiver<Volume>,
    pub volume_control: Control<VolumeCommand>,
    pub tray: watch::Receiver<Tray>,
    pub tray_control: Control<TrayCommand>,
    pub workspaces: watch::Receiver<Workspaces>,
    pub toplevels: watch::Receiver<Toplevels>,
    pub wl: Control<WlCommand>,
    pub apps: watch::Receiver<AppIndex>,
    pub apps_control: Control<AppsCommand>,
    pub badges: watch::Receiver<Badges>,
    pub badges_control: Control<BadgesCommand>,
    pub pins: watch::Receiver<DockPins>,
    pub pins_control: Control<PinEvent>,
    pub launches: watch::Receiver<Launches>,
    pub launch: Control<LaunchCommand>,
    pub results: watch::Receiver<Results>,
    pub search: Control<SearchCommand>,
    pub wallpapers: watch::Receiver<Wallpapers>,
    pub wallpaper_control: Control<WallpaperCommand>,
    pub sound: watch::Receiver<Presence>,
    pub sound_control: Control<SoundCommand>,
}

impl Services {
    /// Start every service on `rt` (the `cosmic_wl` and volume threads on their own threads)
    /// and bundle them. A service whose system half is missing still yields its receiver, at
    /// its default.
    pub fn spawn(rt: &Handle) -> Services {
        todo!("M1 daemon: call each module's spawn; M4 wires results with the five providers")
    }
}
