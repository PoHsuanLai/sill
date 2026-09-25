//! The one value every surface gets through `Root::context`: every service's receiver and
//! control. Clone is cheap (channels are reference-counted).

use tokio::runtime::Handle;
use tokio::sync::watch;

use crate::apps::{AppIndex, AppsCommand};
use crate::badges::{Badges, BadgesCommand};
use crate::battery::{Battery, BatteryCommand};
use crate::bluetooth::{Bluetooth, BluetoothCommand};
use crate::clock::{ClockCommand, ClockText};
use crate::control::Control;
use crate::cosmic_wl::{Toplevels, WlCommand, Workspaces};
use crate::displays::{Displays, DisplaysCommand};
use crate::events::{Agenda, EventsCommand};
use crate::launch::{LaunchCommand, Launches};
use crate::locker::{Lock, LockerCommand};
use crate::network::wifi::{Wifi, WifiCommand};
use crate::network::{Network, NetworkCommand};
use crate::notifications::{History, Notifications, NotificationsCommand};
use crate::now_playing::{Media, MediaCommand};
use crate::pins::{DockPins, PinEvent};
use crate::power_profile::{PowerProfiles, ProfileCommand};
use crate::results::{Results, SearchCommand};
use crate::screenshots::{ShotCommand, Shots};
use crate::session::{Session, SessionCommand};
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
    pub displays: watch::Receiver<Displays>,
    pub displays_control: Control<DisplaysCommand>,
    /// The M5 control center's and power menu's services (FINDINGS "M5 freeze").
    pub wifi: watch::Receiver<Wifi>,
    pub wifi_control: Control<WifiCommand>,
    pub bluetooth: watch::Receiver<Bluetooth>,
    pub bluetooth_control: Control<BluetoothCommand>,
    pub power_profiles: watch::Receiver<PowerProfiles>,
    pub power_profile_control: Control<ProfileCommand>,
    pub media: watch::Receiver<Media>,
    pub media_control: Control<MediaCommand>,
    pub session: watch::Receiver<Session>,
    pub session_control: Control<SessionCommand>,
    /// M7: the borrowed lock screen (FINDINGS "M7"); the session's Lock goes through it.
    pub lock: watch::Receiver<Lock>,
    pub locker_control: Control<LockerCommand>,
    /// M6: the notifications server's live banners and name holder, and its history
    /// (FINDINGS "M6 freeze").
    pub notifications: watch::Receiver<Notifications>,
    pub notification_history: watch::Receiver<History>,
    pub notifications_control: Control<NotificationsCommand>,
    /// M10: calendar events for the Calendar and Up Next widgets, and screenshots for the
    /// floating thumbnail (FINDINGS "M10 freeze"). Both passive until the daemon configures them.
    pub agenda: watch::Receiver<Agenda>,
    pub events_control: Control<EventsCommand>,
    pub shots: watch::Receiver<Shots>,
    pub shots_control: Control<ShotCommand>,
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
        let (displays, displays_control) = crate::displays::spawn(rt, volume.clone());
        let (tray, tray_control) = crate::tray::spawn(rt);
        let (workspaces, toplevels, wl) = crate::cosmic_wl::spawn();
        let (apps, apps_control) = crate::apps::spawn(rt);
        let (badges, badges_control) = crate::badges::spawn(rt);
        let (pins, pins_control) = crate::pins::spawn(rt);
        let (launches, launch) = crate::launch::spawn(rt, apps.clone());
        let (results, search) = crate::results::spawn_with_providers(
            rt,
            crate::results::Wiring {
                apps: &apps,
                toplevels: &toplevels,
                workspaces: &workspaces,
                wl: &wl,
                launch: &launch,
            },
        );
        let (wallpapers, wallpaper_control) = crate::wallpaper::spawn(rt);
        // M6 replaces this with sound::spawn.
        let (sound, sound_control) = inert();
        // M5: each `live::run` is a stub that returns at once until its agent fills it, so these
        // are inert meanwhile without an `inert()` line to swap (FINDINGS "M5 freeze").
        let (wifi, wifi_control) = crate::network::wifi::spawn(rt);
        let (bluetooth, bluetooth_control) = crate::bluetooth::spawn(rt);
        let (power_profiles, power_profile_control) = crate::power_profile::spawn(rt);
        let (media, media_control) = crate::now_playing::spawn(rt);
        let (lock, locker_control) = crate::locker::spawn(rt);
        let (session, session_control) = crate::session::spawn(rt, locker_control.clone());
        // M6: the live loop is a stub that returns at once until lane a fills it, and it is
        // passive (never asks for the bus name) until the daemon sends its first policy.
        let (notifications, notification_history, notifications_control) =
            crate::notifications::spawn(rt);
        // M10: each `live::run` is a stub that returns at once until its lane fills it.
        let (agenda, events_control) = crate::events::spawn(rt);
        let (shots, shots_control) = crate::screenshots::spawn(rt);

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
            displays,
            displays_control,
            wifi,
            wifi_control,
            bluetooth,
            bluetooth_control,
            power_profiles,
            power_profile_control,
            media,
            media_control,
            session,
            session_control,
            lock,
            locker_control,
            notifications,
            notification_history,
            notifications_control,
            agenda,
            events_control,
            shots,
            shots_control,
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
        assert_eq!(
            services.notifications.borrow().server.mode,
            crate::ServerMode::Passive,
            "passive until the daemon's policy"
        );
        assert_eq!(
            services.agenda.borrow().status,
            crate::SourceStatus::Off,
            "no calendar read until the daemon names the sources"
        );
        assert_eq!(
            *services.shots.borrow(),
            Default::default(),
            "nothing watched until the daemon names the folders"
        );
        assert_eq!(
            *services.displays.borrow(),
            Default::default(),
            "inert until a policy"
        );
    }
}
