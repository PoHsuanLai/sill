//! The one value every surface gets through `Root::context`: every service's [`Handle`].
//! Clone is cheap (channels are reference-counted). The daemon starts them
//! (`sill::daemon::services::start`).

use std::sync::Arc;

use sill_search::results::ResultsService;
use sill_services::apps::AppsService;
use sill_services::auth::AuthService;
use sill_services::badges::BadgesService;
use sill_services::battery::BatteryService;
use sill_services::bluetooth::BluetoothService;
use sill_services::clipboard::ClipboardService;
use sill_services::clock::ClockService;
use sill_services::companion::CompanionService;
use sill_services::cosmic_wl::CosmicWlService;
use sill_services::displays::DisplaysService;
use sill_services::env::{Dirs, SessionActions};
use sill_services::events::EventsService;
use sill_services::idle::IdleService;
use sill_services::intents::IntentsService;
use sill_services::kbd_backlight::KbdBacklightService;
use sill_services::keycap::KeycapService;
use sill_services::launch::LaunchService;
use sill_services::locker::LockerService;
use sill_services::network::NetworkService;
use sill_services::notifications::NotificationsService;
use sill_services::now_playing::NowPlayingService;
use sill_services::pins::PinsService;
use sill_services::polkit_agent::PolkitAgentService;
use sill_services::power_profile::PowerProfileService;
use sill_services::screenshots::ScreenshotsService;
use sill_services::service::{Handle, Service};
use sill_services::session::SessionService;
use sill_services::tray::TrayService;
use sill_services::volume::VolumeService;
use sill_services::wallpaper::WallpaperService;
use sill_services::wifi::WifiService;

/// Every running service, each field named after its module. Surfaces read a service through
/// `sill-ui-kit::hooks::use_service` and command it through `use_control`; nothing here is a
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
    /// The keycap keyboard daemon: its connection, and the notices about it.
    pub keycap: Handle<KeycapService>,
    /// sill's seat at intentd: the confirmation sheet it serves, the terminal's grants, the shell's
    /// own actions as `org.quire.Shell`; passive until the daemon says to serve.
    pub intents: Handle<IntentsService>,
    /// The companion: where a summon goes, its answers, the computer-use runs, the undo journal
    /// and the launcher's two lookups; it speaks as the shell on the connection `intents` holds.
    pub companion: Handle<CompanionService>,
    /// The person's directories the services were started with (the icon theme's config, the
    /// widget placements' state folder).
    pub dirs: Dirs,
    /// Where the session service sends chosen actions (logind, or a dry run's record), for the
    /// surfaces' log lines.
    pub session_actions: Arc<dyn SessionActions>,
}

/// A service [`Services`] holds, and which field holds it: what lets a reader name a service by
/// its type alone (`sill-ui-kit`' `use_service::<S>()`).
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
    keycap: KeycapService,
    intents: IntentsService,
    companion: CompanionService,
}
