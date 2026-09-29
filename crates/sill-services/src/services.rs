//! The one value every surface gets through `Root::context`: every service's receiver and
//! control. Clone is cheap (channels are reference-counted).

use std::sync::Arc;

use tokio::runtime::Handle;
use tokio::sync::watch;

use crate::apps::{AppIndex, AppsCommand};
use crate::auth::{Auth, AuthCommand};
use crate::badges::{Badges, BadgesCommand};
use crate::battery::{Battery, BatteryCommand};
use crate::bluetooth::{Bluetooth, BluetoothCommand};
use crate::clipboard::{Clipboard, ClipboardCommand};
use crate::clock::{ClockCommand, ClockText};
use crate::control::Control;
use crate::cosmic_wl::{Toplevels, WlCommand, Workspaces};
use crate::displays::{Displays, DisplaysCommand};
use crate::env::{Dirs, Env, SessionActions};
use crate::events::{Agenda, EventsCommand};
use crate::idle::{Idle, IdleCommand, IdleLinks};
use crate::kbd_backlight::{KbdBacklight, KbdCommand};
use crate::launch::{LaunchCommand, Launches};
use crate::locker::{Lock, LockerCommand};
use crate::network::wifi::{Wifi, WifiCommand};
use crate::network::{Network, NetworkCommand};
use crate::notifications::{History, Notifications, NotificationsCommand};
use crate::now_playing::{Media, MediaCommand};
use crate::pins::{DockPins, PinEvent};
use crate::polkit_agent::{Polkit, PolkitCommand};
use crate::power_profile::{PowerProfiles, ProfileCommand};
use crate::results::{Results, SearchCommand};
use crate::screenshots::{ShotCommand, Shots};
use crate::service::{Handle as ServiceHandle, Service, spawn};
use crate::session::{Session, SessionCommand};
use crate::tray::{Tray, TrayCommand};
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
    pub displays: watch::Receiver<Displays>,
    pub displays_control: Control<DisplaysCommand>,
    /// The control center's and power menu's services.
    pub wifi: watch::Receiver<Wifi>,
    pub wifi_control: Control<WifiCommand>,
    pub bluetooth: watch::Receiver<Bluetooth>,
    pub bluetooth_control: Control<BluetoothCommand>,
    pub power_profiles: watch::Receiver<PowerProfiles>,
    pub power_profile_control: Control<ProfileCommand>,
    pub media: watch::Receiver<Media>,
    pub media_control: Control<MediaCommand>,
    /// The keyboard backlight (design/26 5.2.7): absent on a machine without one.
    pub kbd_backlight: watch::Receiver<KbdBacklight>,
    pub kbd_backlight_control: Control<KbdCommand>,
    pub session: watch::Receiver<Session>,
    pub session_control: Control<SessionCommand>,
    /// sill's own lock screen; the session's Lock goes through it.
    pub lock: watch::Receiver<Lock>,
    pub locker_control: Control<LockerCommand>,
    /// The notifications server's live banners and name holder, and its history.
    pub notifications: watch::Receiver<Notifications>,
    pub notification_history: watch::Receiver<History>,
    pub notifications_control: Control<NotificationsCommand>,
    /// Calendar events for the Calendar and Up Next widgets, and screenshots for the floating
    /// thumbnail. Both passive until the daemon configures them.
    pub agenda: watch::Receiver<Agenda>,
    pub events_control: Control<EventsCommand>,
    pub shots: watch::Receiver<Shots>,
    pub shots_control: Control<ShotCommand>,
    /// The lock screen's password check (idle until a secret is submitted), and the polkit
    /// agent (passive until the daemon's policy).
    pub auth: watch::Receiver<Auth>,
    pub auth_control: Control<AuthCommand>,
    pub polkit: watch::Receiver<Polkit>,
    pub polkit_control: Control<PolkitCommand>,
    /// The clipboard history: passive until the daemon's policy.
    pub clipboard: watch::Receiver<Clipboard>,
    pub clipboard_control: Control<ClipboardCommand>,
    /// sill's own idle: passive until the daemon's policy, and `Off` anywhere but a sill
    /// session.
    pub idle: watch::Receiver<Idle>,
    pub idle_control: Control<IdleCommand>,
    /// The person's directories the services were started with (the icon theme's config, the
    /// widget placements' state folder).
    pub dirs: Dirs,
    /// Where the session service sends chosen actions (logind, or a dry run's record), for the
    /// surfaces' log lines.
    pub session_actions: Arc<dyn SessionActions>,
}

impl Services {
    /// Start every service on `rt` (the `cosmic_wl` and volume threads on their own threads)
    /// against `env`'s seams, and bundle them. A service whose system half is missing still
    /// yields its receiver, at its default.
    pub fn spawn(rt: &Handle, env: &Env) -> Services {
        let system = || env.buses.system.clone();
        let (clock, clock_control) = parts(spawn::<crate::clock::ClockService>(rt, env, ()));
        let (battery, battery_control) =
            parts(spawn::<crate::battery::BatteryService>(rt, env, ()));
        let (network, network_control) =
            parts(spawn::<crate::network::NetworkService>(rt, env, ()));
        let (volume, volume_control) = parts(spawn::<crate::volume::VolumeService>(rt, env, ()));
        let (displays, displays_control) = parts(spawn::<crate::displays::DisplaysService>(
            rt,
            env,
            volume.clone(),
        ));
        let (tray, tray_control) = parts(spawn::<crate::tray::TrayService>(rt, env, ()));
        let (workspaces, toplevels, wl) = crate::cosmic_wl::spawn(env.wayland.clone());
        let (apps, apps_control) = crate::apps::spawn(rt, &env.dirs);
        let (badges, badges_control) = parts(spawn::<crate::badges::BadgesService>(rt, env, ()));
        let (pins, pins_control) = parts(spawn::<crate::pins::PinsService>(rt, env, ()));
        let (launches, launch) =
            crate::launch::spawn(rt, crate::launch::Setup::of(env), apps.clone());
        let (clipboard, clipboard_control) = crate::clipboard::spawn(
            rt,
            crate::clipboard::Setup {
                display: env.wayland.clone(),
            },
        );
        let (results, search) = crate::results::spawn_with_providers(
            rt,
            crate::results::Wiring {
                apps: &apps,
                toplevels: &toplevels,
                workspaces: &workspaces,
                clipboard: &clipboard,
                wl: &wl,
                launch: &launch,
                buses: &env.buses,
                dirs: &env.dirs,
            },
        );
        let (wallpapers, wallpaper_control) = crate::wallpaper::spawn(rt, &env.dirs);
        let (wifi, wifi_control) = parts(spawn::<crate::network::wifi::WifiService>(rt, env, ()));
        let (bluetooth, bluetooth_control) =
            parts(spawn::<crate::bluetooth::BluetoothService>(rt, env, ()));
        let (power_profiles, power_profile_control) = parts(spawn::<
            crate::power_profile::PowerProfileService,
        >(rt, env, ()));
        let (media, media_control) =
            parts(spawn::<crate::now_playing::NowPlayingService>(rt, env, ()));
        let (kbd_backlight, kbd_backlight_control) = parts(spawn::<
            crate::kbd_backlight::KbdBacklightService,
        >(rt, env, ()));
        let (lock, locker_control) = crate::locker::spawn(rt, system(), env.dirs.runtime.clone());
        let (session, session_control) = crate::session::spawn(
            rt,
            system(),
            Arc::clone(&env.session_actions),
            locker_control.clone(),
        );
        // Passive (never asks for the bus name) until the daemon sends its first policy.
        let (notifications, notification_history, notifications_control) =
            crate::notifications::spawn(rt, crate::notifications::Setup::of(env));
        let (agenda, events_control) = crate::events::spawn(rt, &env.dirs, Arc::clone(&env.clock));
        let (shots, shots_control) = crate::screenshots::spawn(rt, &env.dirs);
        let (auth, auth_control) = crate::auth::spawn(rt, crate::auth::AccountPlace::of(env));
        let (polkit, polkit_control) = crate::polkit_agent::spawn(rt, env.polkit.clone());
        let (idle, idle_control) = crate::idle::spawn(
            rt,
            IdleLinks {
                lock: lock.clone(),
                battery: battery.clone(),
                locker: locker_control.clone(),
                auth: auth_control.clone(),
            },
            crate::idle::Setup::of(env),
        );

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
            kbd_backlight,
            kbd_backlight_control,
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
            auth,
            auth_control,
            polkit,
            polkit_control,
            clipboard,
            clipboard_control,
            idle,
            idle_control,
            dirs: env.dirs.clone(),
            session_actions: Arc::clone(&env.session_actions),
        }
    }
}

/// A handle's two halves, for the fields above.
fn parts<S: Service>(handle: ServiceHandle<S>) -> (watch::Receiver<S::State>, Control<S::Command>) {
    (handle.state, handle.control)
}

#[cfg(test)]
mod tests {
    use super::Services;
    use crate::env::scratch::Scratch;

    /// `Services::spawn` must never panic — the daemon has to reach the host connect and the
    /// IPC server — and every passive service holds its default (a service without its bus or
    /// compositor degrades to its own default, per `ServiceError`'s doc); the clock reads the
    /// time at once (`Thu 24 Sep 14:05`: four words).
    #[test]
    fn spawn_does_not_panic_and_passive_services_hold_their_defaults() {
        let rt = tokio::runtime::Runtime::new().unwrap_or_else(|e| panic!("{e}"));
        // No bus, no compositor, every directory in a scratch folder: nothing here reaches the
        // person's session or files.
        let scratch = Scratch::new("services-spawn");
        let services = Services::spawn(rt.handle(), &scratch.env());

        assert_eq!(services.clock.borrow().text.split_whitespace().count(), 4);
        assert_eq!(*services.apps.borrow(), Default::default());
        assert_eq!(*services.badges.borrow(), Default::default());
        assert_eq!(*services.pins.borrow(), Default::default());
        assert_eq!(*services.launches.borrow(), Default::default());
        assert_eq!(*services.results.borrow(), Default::default());
        assert_eq!(*services.wallpapers.borrow(), Default::default());
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
        assert_eq!(
            services.polkit.borrow().registration,
            crate::polkit_agent::Registration::Passive,
            "no agent registered until the daemon's policy"
        );
        assert_eq!(
            services.auth.borrow().phase,
            crate::AuthPhase::Idle,
            "nothing checked until a secret is submitted"
        );
        assert_eq!(
            services.clipboard.borrow().status.mode,
            crate::clipboard::HistoryMode::Off,
            "no clipboard read until the daemon's policy"
        );
        assert_eq!(
            services.idle.borrow().mode,
            crate::idle::IdleMode::Off(crate::idle::OffReason::NoPolicy),
            "no idle notification, display or logind touched until the daemon's policy"
        );
    }
}
