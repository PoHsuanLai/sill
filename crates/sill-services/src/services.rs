//! The one value every surface gets through `Root::context`: every service's receiver and
//! control. Clone is cheap (channels are reference-counted).

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
    ///
    /// A service whose `spawn` is still a freeze stub is not called (a `todo!()` would panic
    /// `sill run` before it ever reaches the host or the IPC server, and the daemon has to
    /// start today): [`inert`] builds its pair locally instead, one line per service naming the
    /// wave that replaces it. Every M1 service is real: `clock` (M1 bar-ui), `battery`,
    /// `network`, `volume` (M1 svc-dbus), `tray` (M1 svc-tray) and `cosmic_wl` (M1 svc-cosmic);
    /// the M2-M6 services below are still stubs.
    pub fn spawn(rt: &Handle) -> Services {
        let (clock, clock_control) = crate::clock::spawn(rt);
        let (battery, battery_control) = crate::battery::spawn(rt);
        let (network, network_control) = crate::network::spawn(rt);
        let (volume, volume_control) = crate::volume::spawn(rt);
        let (tray, tray_control) = crate::tray::spawn(rt);
        let (workspaces, toplevels, wl) = crate::cosmic_wl::spawn();
        // M2 apps-index replaces this with apps::spawn.
        let (apps, apps_control) = inert();
        let (badges, badges_control) = crate::badges::spawn(rt);
        // M2 launch (dock_config) replaces this with pins::spawn.
        let (pins, pins_control) = inert();
        // M2 launch replaces this with launch::spawn(rt, apps).
        let (launches, launch) = inert();
        // M4 launcher-core replaces this with results::spawn(rt, the five providers).
        let (results, search) = inert();
        // M2 wallpaper replaces this with wallpaper::spawn.
        let (wallpapers, wallpaper_control) = inert();
        // M6 replaces this with sound::spawn.
        let (sound, sound_control) = inert();

        Services {
            clock,
            clock_control,
            battery,
            battery_control,
            network,
            network_control,
            volume,
            volume_control,
            tray,
            tray_control,
            workspaces,
            toplevels,
            wl,
            apps,
            apps_control,
            badges,
            badges_control,
            pins,
            pins_control,
            launches,
            launch,
            results,
            search,
            wallpapers,
            wallpaper_control,
            sound,
            sound_control,
        }
    }
}

/// A receiver fixed at `T::default()` and a `Control` whose commands go nowhere, for a service
/// whose `spawn` is still a freeze stub. The command channel's receiving half is dropped at
/// once (nobody is ever going to drain it), so `Control::send` against the returned `Control`
/// answers `ServiceError::Gone` rather than queuing forever.
fn inert<T: Default, C>() -> (watch::Receiver<T>, Control<C>) {
    let (_tx, rx) = watch::channel(T::default());
    let (control, _rx) = Control::channel();
    (rx, control)
}

#[cfg(test)]
mod tests {
    use super::Services;

    /// `Services::spawn` must never panic — the daemon has to reach the host connect and the
    /// IPC server before every milestone lands — and every receiver still stubbed out holds its
    /// default (a service that is real, `battery`/`network`/`volume`/`tray`/`cosmic_wl`, degrades
    /// to its own default without a bus or a compositor, per `ServiceError`'s doc); the clock
    /// reads the time at once (`Thu 24 Sep 14:05`: four words).
    #[test]
    fn spawn_does_not_panic_and_every_stub_holds_its_default() {
        let rt = tokio::runtime::Runtime::new().unwrap_or_else(|e| panic!("{e}"));
        let services = Services::spawn(rt.handle());

        assert_eq!(services.clock.borrow().text.split_whitespace().count(), 4);
        assert_eq!(*services.apps.borrow(), Default::default());
        assert_eq!(*services.badges.borrow(), Default::default());
        assert_eq!(*services.pins.borrow(), Default::default());
        assert_eq!(*services.launches.borrow(), Default::default());
        assert_eq!(*services.results.borrow(), Default::default());
        assert_eq!(*services.wallpapers.borrow(), Default::default());
        assert_eq!(*services.sound.borrow(), Default::default());
    }
}
