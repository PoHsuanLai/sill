//! The one value every surface gets through `Root::context`: every service's [`Handle`].
//! Clone is cheap (channels are reference-counted).

use std::sync::Arc;

use tokio::runtime::Handle as Runtime;

use crate::apps::AppsService;
use crate::auth::AuthService;
use crate::badges::BadgesService;
use crate::battery::BatteryService;
use crate::bluetooth::BluetoothService;
use crate::clipboard::ClipboardService;
use crate::clock::ClockService;
use crate::cosmic_wl::CosmicWlService;
use crate::displays::DisplaysService;
use crate::env::{Dirs, Env, SessionActions};
use crate::events::EventsService;
use crate::idle::{IdleLinks, IdleService};
use crate::kbd_backlight::KbdBacklightService;
use crate::launch::LaunchService;
use crate::locker::LockerService;
use crate::network::NetworkService;
use crate::notifications::NotificationsService;
use crate::now_playing::NowPlayingService;
use crate::pins::PinsService;
use crate::polkit_agent::PolkitAgentService;
use crate::power_profile::PowerProfileService;
use crate::results::{ResultsLinks, ResultsService, Sources};
use crate::screenshots::ScreenshotsService;
use crate::service::{Handle, Service, spawn};
use crate::session::SessionService;
use crate::tray::TrayService;
use crate::volume::VolumeService;
use crate::wallpaper::WallpaperService;
use crate::wifi::WifiService;

/// Every running service, each field named after its module. Surfaces read a service through
/// `sill-surfaces::hooks::use_service` and command it through `use_control`; nothing here is a
/// Dioxus signal (review rule: never pass a Signal through `Root::context`).
#[derive(Debug, Clone)]
pub struct Services {
    pub clock: Handle<ClockService>,
    pub battery: Handle<BatteryService>,
    pub network: Handle<NetworkService>,
    pub wifi: Handle<WifiService>,
    pub volume: Handle<VolumeService>,
    pub bluetooth: Handle<BluetoothService>,
    /// The keyboard backlight (design/26 5.2.7): absent on a machine without one.
    pub kbd_backlight: Handle<KbdBacklightService>,
    pub power_profile: Handle<PowerProfileService>,
    pub now_playing: Handle<NowPlayingService>,
    pub badges: Handle<BadgesService>,
    pub tray: Handle<TrayService>,
    pub pins: Handle<PinsService>,
    pub apps: Handle<AppsService>,
    pub cosmic_wl: Handle<CosmicWlService>,
    pub launch: Handle<LaunchService>,
    /// The clipboard history: passive until the daemon's policy.
    pub clipboard: Handle<ClipboardService>,
    pub results: Handle<ResultsService>,
    pub wallpaper: Handle<WallpaperService>,
    /// Inert until the daemon's first policy.
    pub displays: Handle<DisplaysService>,
    /// sill's own lock screen; the session's Lock goes through it.
    pub locker: Handle<LockerService>,
    pub session: Handle<SessionService>,
    /// The notifications server's live banners and name holder, and its history.
    pub notifications: Handle<NotificationsService>,
    /// Calendar events for the Calendar and Up Next widgets, and screenshots for the floating
    /// thumbnail. Both passive until the daemon configures them.
    pub events: Handle<EventsService>,
    pub screenshots: Handle<ScreenshotsService>,
    /// The lock screen's password check (idle until a secret is submitted), and the polkit
    /// agent (passive until the daemon's policy).
    pub auth: Handle<AuthService>,
    pub polkit_agent: Handle<PolkitAgentService>,
    /// sill's own idle: passive until the daemon's policy, and `Off` anywhere but a sill
    /// session.
    pub idle: Handle<IdleService>,
    /// The person's directories the services were started with (the icon theme's config, the
    /// widget placements' state folder).
    pub dirs: Dirs,
    /// Where the session service sends chosen actions (logind, or a dry run's record), for the
    /// surfaces' log lines.
    pub session_actions: Arc<dyn SessionActions>,
}

impl Services {
    /// Start every service on `rt` (the `cosmic_wl` and volume threads on their own threads)
    /// against `env`'s seams, each after the ones it depends on. A service whose system half is
    /// missing still yields its receiver, at its default.
    pub fn spawn(rt: &Runtime, env: &Env) -> Services {
        let volume = spawn::<VolumeService>(rt, env, ());
        let battery = spawn::<BatteryService>(rt, env, ());
        let apps = spawn::<AppsService>(rt, env, ());
        let cosmic_wl = spawn::<CosmicWlService>(rt, env, ());
        let launch = spawn::<LaunchService>(rt, env, apps.state.clone());
        let clipboard = spawn::<ClipboardService>(rt, env, ());
        let results = spawn::<ResultsService>(
            rt,
            env,
            Sources::Services(ResultsLinks {
                apps: apps.state.clone(),
                cosmic_wl: cosmic_wl.state.clone(),
                clipboard: clipboard.state.clone(),
                wl: cosmic_wl.control.clone(),
                launch: launch.control.clone(),
            }),
        );
        let locker = spawn::<LockerService>(rt, env, ());
        let auth = spawn::<AuthService>(rt, env, ());
        let idle = spawn::<IdleService>(
            rt,
            env,
            IdleLinks {
                lock: locker.state.clone(),
                battery: battery.state.clone(),
                locker: locker.control.clone(),
                auth: auth.control.clone(),
            },
        );
        Services {
            clock: spawn(rt, env, ()),
            network: spawn(rt, env, ()),
            wifi: spawn(rt, env, ()),
            displays: spawn(rt, env, volume.state.clone()),
            bluetooth: spawn(rt, env, ()),
            kbd_backlight: spawn(rt, env, ()),
            power_profile: spawn(rt, env, ()),
            now_playing: spawn(rt, env, ()),
            badges: spawn(rt, env, ()),
            tray: spawn(rt, env, ()),
            pins: spawn(rt, env, ()),
            wallpaper: spawn(rt, env, ()),
            session: spawn(rt, env, locker.control.clone()),
            // Passive (never asks for the bus name) until the daemon sends its first policy.
            notifications: spawn(rt, env, ()),
            events: spawn(rt, env, ()),
            screenshots: spawn(rt, env, ()),
            polkit_agent: spawn(rt, env, ()),
            volume,
            battery,
            apps,
            cosmic_wl,
            launch,
            clipboard,
            results,
            locker,
            auth,
            idle,
            dirs: env.dirs.clone(),
            session_actions: Arc::clone(&env.session_actions),
        }
    }
}

/// A service [`Services`] holds, and which field holds it: what lets a reader name a service by
/// its type alone (`sill-surfaces`' `use_service::<S>()`).
pub trait Member: Service {
    fn of(services: &Services) -> &Handle<Self>;
}

/// `impl Member` for each `field: Service` pair.
macro_rules! members {
    ($($field:ident: $service:ty),* $(,)?) => {
        $(impl Member for $service {
            fn of(services: &Services) -> &Handle<Self> {
                &services.$field
            }
        })*
    };
}

members! {
    clock: ClockService,
    battery: BatteryService,
    network: NetworkService,
    wifi: WifiService,
    volume: VolumeService,
    bluetooth: BluetoothService,
    kbd_backlight: KbdBacklightService,
    power_profile: PowerProfileService,
    now_playing: NowPlayingService,
    badges: BadgesService,
    tray: TrayService,
    pins: PinsService,
    apps: AppsService,
    cosmic_wl: CosmicWlService,
    launch: LaunchService,
    clipboard: ClipboardService,
    results: ResultsService,
    wallpaper: WallpaperService,
    displays: DisplaysService,
    locker: LockerService,
    session: SessionService,
    notifications: NotificationsService,
    events: EventsService,
    screenshots: ScreenshotsService,
    auth: AuthService,
    polkit_agent: PolkitAgentService,
    idle: IdleService,
}

#[cfg(test)]
mod tests {
    use super::Services;
    use sill_testkit::TempDir;

    /// `Services::spawn` must never panic — the daemon has to reach the host connect and the
    /// IPC server — and every passive service holds its default (a service without its bus or
    /// compositor degrades to its own default, per `ServiceError`'s doc); the clock reads the
    /// time at once (`Thu 24 Sep 14:05`: four words).
    #[test]
    fn spawn_does_not_panic_and_passive_services_hold_their_defaults() {
        let rt = tokio::runtime::Runtime::new().unwrap_or_else(|e| panic!("{e}"));
        // No bus, no compositor, every directory in a scratch folder: nothing here reaches the
        // person's session or files.
        let scratch = TempDir::new("services-spawn");
        let services = Services::spawn(rt.handle(), &crate::env::Env::isolated(scratch.path()));

        assert_eq!(
            services
                .clock
                .state
                .borrow()
                .text
                .split_whitespace()
                .count(),
            4
        );
        assert_eq!(*services.apps.state.borrow(), Default::default());
        assert_eq!(*services.badges.state.borrow(), Default::default());
        assert_eq!(*services.pins.state.borrow(), Default::default());
        assert_eq!(*services.launch.state.borrow(), Default::default());
        assert_eq!(*services.results.state.borrow(), Default::default());
        assert_eq!(*services.wallpaper.state.borrow(), Default::default());
        assert_eq!(
            services.notifications.state.borrow().live.server.mode,
            sill_model::notifications::ServerMode::Passive,
            "passive until the daemon's policy"
        );
        assert_eq!(
            services.events.state.borrow().status,
            sill_model::events::SourceStatus::Off,
            "no calendar read until the daemon names the sources"
        );
        assert_eq!(
            *services.screenshots.state.borrow(),
            Default::default(),
            "nothing watched until the daemon names the folders"
        );
        assert_eq!(
            *services.displays.state.borrow(),
            Default::default(),
            "inert until a policy"
        );
        assert_eq!(
            services.polkit_agent.state.borrow().registration,
            sill_model::polkit_agent::Registration::Passive,
            "no agent registered until the daemon's policy"
        );
        assert_eq!(
            services.auth.state.borrow().phase,
            sill_model::auth::AuthPhase::Idle,
            "nothing checked until a secret is submitted"
        );
        assert_eq!(
            services.clipboard.state.borrow().status.mode,
            sill_model::clipboard::HistoryMode::Off,
            "no clipboard read until the daemon's policy"
        );
        assert_eq!(
            services.idle.state.borrow().mode,
            sill_model::idle::IdleMode::Off(sill_model::idle::OffReason::NoPolicy),
            "no idle notification, display or logind touched until the daemon's policy"
        );
    }
}
