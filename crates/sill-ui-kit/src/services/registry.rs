//! Every service, declared once, in the order they start: what it is, what it needs from the
//! ones above it, and (behind `harness`) the state its fake starts at when that is not its
//! default. To add a service, add its line here (after the services in its deps); the
//! [`Services`] field, its [`Member`](super::Member) impl, the spawn and the fake follow.

use std::sync::Arc;

use sill_model::desktop::DesktopView;
use sill_search::results::{ResultsLinks, ResultsService, Sources};
use sill_services::accounts::sheet::{AccountsSheetLinks, AccountsSheetService};
use sill_services::accounts::{AccountsLinks, AccountsService};
use sill_services::apps::AppsService;
use sill_services::auth::AuthService;
use sill_services::badges::BadgesService;
use sill_services::battery::BatteryService;
use sill_services::bluetooth::BluetoothService;
use sill_services::capture_pick::{CaptureLinks, CaptureService};
use sill_services::clipboard::ClipboardService;
use sill_services::clock::ClockService;
use sill_services::companion::{CompanionLinks, CompanionService};
use sill_services::displays::DisplaysService;
use sill_services::env::{Dirs, SessionActions};
use sill_services::events::EventsService;
use sill_services::guest_asks::{GuestAsksLinks, GuestAsksService};
use sill_services::idle::{IdleLinks, IdleService};
use sill_services::intents::{IntentsLinks, IntentsService};
use sill_services::kbd_backlight::KbdBacklightService;
use sill_services::keycap::{KeycapLinks, KeycapService};
use sill_services::launch::{LaunchLinks, LaunchService};
use sill_services::locker::LockerService;
use sill_services::network::NetworkService;
use sill_services::night_shift::NightShiftService;
use sill_services::notifications::NotificationsService;
use sill_services::now_playing::NowPlayingService;
use sill_services::pins::PinsService;
use sill_services::polkit_agent::PolkitAgentService;
use sill_services::power_profile::PowerProfileService;
use sill_services::screenshots::ScreenshotsService;
use sill_services::session::{SessionLinks, SessionService};
use sill_services::sync_hold::{SyncHoldLinks, SyncHoldService};
use sill_services::tray::TrayService;
use sill_services::voice::{VoiceLinks, VoiceService};
use sill_services::volume::VolumeService;
use sill_services::wallpaper::WallpaperService;
use sill_services::wifi::WifiService;
use sill_services::work_areas::WorkAreaService;
use sill_services::workspace_looks::WorkspaceLooksService;
use sill_services::{AppMenuDeps, AppMenuService, MenuLinks};
use sill_services::{WindowsLinks, WindowsService};
use tokio::sync::watch;

use super::Wiring;
use crate::app_menu::menu_name_of;
use crate::dock_rects::DockRects;

services! {
    start(rt, env, wiring: Wiring);
    {
        apps: AppsService = ();
        /// The notifications server's live banners and name holder, and its history. Passive until
        /// the daemon's policy.
        notifications: NotificationsService = wiring.answers.clone();
        launch: LaunchService = LaunchLinks {
            apps: apps.state.clone(),
            notifications: notifications.control.clone(),
        };
        /// The companion: where a summon goes, its answers, the computer-use runs, the undo journal
        /// and the launcher's two lookups; it speaks as the shell on the connection `intents` holds.
        companion: CompanionService = CompanionLinks {
            seat: wiring.seat_view.clone(),
            summons: wiring.summons,
            launch: launch.control.clone(),
        };
        /// Hold-to-talk: where the hold stands, the words heard so far and the consent sheet; it
        /// speaks to voiced as the shell, and asks the companion what it heard.
        voice: VoiceService = VoiceLinks {
            seat: wiring.seat_view.clone(),
            companion: companion.control.clone(),
            consents: wiring.consents,
        };
        volume: VolumeService = () => crate::harness::fake::volume();
        /// Inert until the daemon's first policy.
        displays: DisplaysService = volume.state.clone();
        /// The screen-sharing picker transom asks (`org.quire.Shell1.PickCaptureSource`); served on
        /// the connection `intents` holds, passive until the daemon says to serve.
        capture: CaptureService = CaptureLinks {
            seat: wiring.seat_view.clone(),
            displays: displays.state.clone(),
            desktop: wiring.desktop.clone(),
            apps: apps.state.clone(),
            answers: wiring.permission_answers,
        };
        battery: BatteryService = () => crate::harness::fake::battery();
        windows: WindowsService = WindowsLinks {
            desktop: wiring.desktop_tx,
        } => sill_model::windows::WindowModel {
            workspaces: crate::harness::fake::workspaces(),
            toplevels: crate::harness::fake::toplevels(),
            ..sill_model::windows::WindowModel::default()
        };
        /// The clipboard history: passive until the daemon's policy.
        clipboard: ClipboardService = ();
        /// sill's own lock screen; the session's Lock goes through it.
        locker: LockerService = ();
        session: SessionService = SessionLinks {
            locker: locker.control.clone(),
            windows: windows.state.clone(),
            wl: windows.control.clone(),
            apps: apps.state.clone(),
            notifications: notifications.control.clone(),
        };
        results: ResultsService = Sources::Services(ResultsLinks {
            apps: apps.state.clone(),
            windows: windows.state.clone(),
            clipboard: clipboard.state.clone(),
            companion: companion.state.clone(),
            companion_control: companion.control.clone(),
            session: session.state.clone(),
            wl: windows.control.clone(),
        });
        /// The lock screen's password check (idle until a secret is submitted).
        auth: AuthService = ();
        /// sill's own idle: passive until the daemon's policy, and `Off` anywhere but a sill
        /// session.
        idle: IdleService = IdleLinks {
            lock: locker.state.clone(),
            battery: battery.state.clone(),
            locker: locker.control.clone(),
            auth: auth.control.clone(),
        };
        /// Screenshots for the floating thumbnail; passive until the daemon configures it.
        screenshots: ScreenshotsService = ();
        /// The keycap keyboard daemon: its connection, and the notices about it.
        keycap: KeycapService = KeycapLinks {
            windows: windows.state.clone(),
            apps: apps.state.clone(),
            notifications: notifications.control.clone(),
            notes: notifications.state.clone(),
        };
        /// The accounts that need signing in again, and the notices that say so.
        accounts: AccountsService = AccountsLinks {
            notifications: notifications.control.clone(),
            notes: notifications.state.clone(),
            answers: wiring.answers.clone(),
        };
        /// The accounts sheet accountd opens on `org.quire.AccountsSheet1`; passive until the daemon
        /// says to serve.
        accounts_sheet: AccountsSheetService = AccountsSheetLinks {
            launch: launch.control.clone(),
        };
        /// Datasets whose removal syncd holds back, and the notices that say so.
        sync_hold: SyncHoldService = SyncHoldLinks {
            notifications: notifications.control.clone(),
            notes: notifications.state.clone(),
            answers: wiring.answers.clone(),
            launch: launch.control.clone(),
        };
        /// Computers of the person's asking to borrow this one's models, and the notices that ask.
        guest_asks: GuestAsksService = GuestAsksLinks {
            notifications: notifications.control.clone(),
            notes: notifications.state.clone(),
            answers: wiring.answers.clone(),
        };
        /// casement's night light, asked for by the person (the control center's Night Shift); it
        /// reads and writes casement's settings file, and tells casement which monitors the
        /// displays service warms itself.
        night_shift: NightShiftService = displays.state.clone();
        clock: ClockService = () => crate::harness::fake::clock();
        network: NetworkService = () => crate::harness::fake::network();
        wifi: WifiService = () => crate::harness::fake::wifi();
        bluetooth: BluetoothService = () => crate::harness::fake::bluetooth();
        /// The keyboard backlight (design/26 5.2.7): absent on a machine without one.
        kbd_backlight: KbdBacklightService = ();
        power_profile: PowerProfileService = () => crate::harness::fake::power_profiles();
        now_playing: NowPlayingService = ();
        badges: BadgesService = ();
        tray: TrayService = ();
        /// The menus the focused apps serve (`com.canonical.dbusmenu`): empty until a bar names an app
        /// that has one.
        app_menu: AppMenuService = AppMenuDeps {
            name_of: menu_name_of,
            windows: windows.state.clone(),
        };
        pins: PinsService = apps.state.clone();
        wallpaper: WallpaperService = ();
        /// Calendar events for the Calendar and Up Next widgets; passive until the daemon
        /// configures it.
        events: EventsService = ();
        /// The polkit agent (passive until the daemon's policy).
        polkit_agent: PolkitAgentService = ();
        /// sill's seat at intentd: the confirmation sheet it serves, the terminal's grants, the shell's
        /// own actions as `org.quire.Shell`; passive until the daemon says to serve.
        intents: IntentsService = IntentsLinks {
            calls: wiring.shell_calls,
            menus: MenuLinks {
                toplevels: windows.state.clone(),
                wl: windows.control.clone(),
                name_of: menu_name_of,
            },
            shots: screenshots.state.clone(),
            seat: wiring.seat_publisher,
        };
        /// Each output's work area and scale, served on `org.quire.Outputs1`; passive until the daemon
        /// says to serve.
        work_areas: WorkAreaService = ();
        /// Each workspace's name, output and `SpaceLook`, served on `org.quire.Workspaces1`; passive
        /// until the daemon says to serve.
        workspace_looks: WorkspaceLooksService = ();
    }
    {
        /// The desktop as the compositor's own protocol describes it (apps, windows, Spaces),
        /// projected by the windows service from the state `windows` is projected from. The one
        /// channel every reader of the desktop's view holds; empty on any other compositor.
        desktop: watch::Receiver<DesktopView> = wiring.desktop
            => watch::Sender::new(DesktopView::default()).subscribe();
        /// The person's directories the services were started with (the icon theme's config, the
        /// widget placements' state folder).
        dirs: Dirs = env.dirs.clone()
            => Dirs::under(std::path::Path::new("/nonexistent/sill-ui-kit-test"));
        /// Where the docks' icons rest, for the compositor's minimise targets.
        dock_rects: DockRects = DockRects::default() => DockRects::default();
        /// Where the session service sends chosen actions (logind, or a dry run's record), for the
        /// surfaces' log lines.
        session_actions: Arc<dyn SessionActions> = Arc::clone(&env.session_actions)
            => Arc::new(sill_services::env::DryRunActions::new(
                sill_services::env::DryRunOffers::Everything,
                None,
            ));
    }
}
