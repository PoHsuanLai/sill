//! Starting every service, each after the ones in its `Deps`, into the [`Services`] every
//! surface reads.

use std::sync::Arc;

use sill_model::companion::SummonOut;
use sill_model::intents::IntentsCommand;
use sill_model::locker::LockerCommand;
use sill_model::polkit_agent::{AgentPolicy, PolkitCommand};
use sill_search::results::{ResultsLinks, ResultsService, Sources};
use sill_services::apps::AppsService;
use sill_services::auth::AuthService;
use sill_services::battery::BatteryService;
use sill_services::clipboard::ClipboardService;
use sill_services::companion::{CompanionLinks, CompanionService};
use sill_services::cosmic_wl::CosmicWlService;
use sill_services::env::Env;
use sill_services::idle::{IdleLinks, IdleService};
use sill_services::intents::{IntentsLinks, IntentsService, ShellCall};
use sill_services::keycap::{KeycapLinks, KeycapService};
use sill_services::launch::LaunchService;
use sill_services::locker::LockerService;
use sill_services::notifications::NotificationsService;
use sill_services::seat::seat;
use sill_services::service::spawn;
use sill_services::volume::VolumeService;
use sill_services::{AppMenuDeps, AppMenuService, MenuLinks};
use sill_ui_kit::app_menu::menu_name_of;
use sill_ui_kit::links::Links;
use sill_ui_kit::services::Services;
use sill_ui_kit::settings::SettingsFeed;
use tokio::runtime::Handle as Runtime;
use tokio::sync::mpsc::UnboundedSender;

use crate::policy::{
    calendar::CalendarBridge, clipboard::ClipboardBridge, companion::CompanionBridge,
    displays::DisplaysBridge, follow, idle::IdleBridge, keyboard::KeyboardBridge,
    notifications::NotificationsBridge, screenshots::ScreenshotsBridge, search::SearchBridge,
};

/// Start every service on `rt` (the `cosmic_wl` and volume threads on their own threads)
/// against `env`'s seams, each after the ones it depends on. A service whose system half is
/// missing still yields its receiver, at its default. `shell_calls` is where the shell's own
/// actions go, `summons` where a summon's launcher effects go.
pub(crate) fn start(
    rt: &Runtime,
    env: &Env,
    shell_calls: UnboundedSender<ShellCall>,
    summons: UnboundedSender<SummonOut>,
) -> Services {
    // The connection the intents service claims `org.quire.Shell` on, which the companion speaks
    // on as the launcher.
    let (seat_publisher, seat_view) = seat();
    let companion = spawn::<CompanionService>(
        rt,
        env,
        CompanionLinks {
            seat: seat_view,
            summons,
        },
    );
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
            companion: companion.state.clone(),
            companion_control: companion.control.clone(),
            wl: cosmic_wl.control.clone(),
            launch: launch.control.clone(),
        }),
    );
    // Passive (never asks for the bus name) until the daemon sends its first policy.
    let notifications = spawn::<NotificationsService>(rt, env, ());
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
    let keycap = spawn::<KeycapService>(
        rt,
        env,
        KeycapLinks {
            cosmic_wl: cosmic_wl.state.clone(),
            apps: apps.state.clone(),
            notifications: notifications.control.clone(),
            notes: notifications.state.clone(),
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
        app_menu: spawn::<AppMenuService>(
            rt,
            env,
            AppMenuDeps {
                name_of: menu_name_of,
            },
        ),
        pins: spawn(rt, env, ()),
        wallpaper: spawn(rt, env, ()),
        session: spawn(rt, env, locker.control.clone()),
        notifications,
        events: spawn(rt, env, ()),
        screenshots: spawn(rt, env, ()),
        polkit_agent: spawn(rt, env, ()),
        intents: spawn::<IntentsService>(
            rt,
            env,
            IntentsLinks {
                calls: shell_calls,
                menus: MenuLinks {
                    toplevels: cosmic_wl.state.clone(),
                    wl: cosmic_wl.control.clone(),
                    name_of: menu_name_of,
                },
                seat: seat_publisher,
            },
        ),
        companion,
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
        keycap,
        dirs: env.dirs.clone(),
        session_actions: Arc::clone(&env.session_actions),
    }
}

/// Tell each service that is passive until told what it should do: sill's own lock and polkit
/// agent, and the policies the settings decide (the first one sent ends a service's passive
/// start, and each is sent again whenever its answer changes).
pub(crate) fn follow_settings(
    rt: &Runtime,
    feed: &SettingsFeed,
    links: &Links,
    services: &Services,
    env: &Env,
) {
    let settings = || feed.settings.clone();
    // The notifications service's policy from the settings and the focus mode.
    follow(
        rt,
        settings(),
        NotificationsBridge {
            focus: links.focus.mode(),
        },
        services.notifications.control.clone(),
    );
    // The lock, the polkit agent and the seat at intentd (the confirmation sheet, the terminal's
    // grants, `org.quire.Shell`) are sill's own.
    let _ = services.locker.control.send(LockerCommand::Configure);
    let _ = services.intents.control.send(IntentsCommand::Serve);
    let _ = services
        .polkit_agent
        .control
        .send(PolkitCommand::Policy(AgentPolicy::Own));
    // sill's own idle follows `idle.*` and `session.lock_grace_s`, and is `Off` anywhere but a
    // sill session with no cosmic-idle.
    let idle = IdleBridge {
        times: env.idle_times,
    };
    follow(rt, settings(), idle, services.idle.control.clone());
    // The launcher's providers and the clipboard history follow `launcher.*`.
    follow(
        rt,
        settings(),
        SearchBridge,
        services.results.control.clone(),
    );
    follow(
        rt,
        settings(),
        ClipboardBridge,
        services.clipboard.control.clone(),
    );
    // The displays follow `display.*`.
    follow(
        rt,
        settings(),
        DisplaysBridge,
        services.displays.control.clone(),
    );
    // The companion's double tap and summon follow `companion.*`.
    follow(
        rt,
        settings(),
        CompanionBridge,
        services.companion.control.clone(),
    );
    // keycap's per-app profiles follow `keyboard.overrides`.
    follow(
        rt,
        settings(),
        KeyboardBridge,
        services.keycap.control.clone(),
    );
    // The calendar's sources and the screenshots' folders and tool.
    let home = env.dirs.home.clone();
    let calendar = CalendarBridge { home: home.clone() };
    follow(rt, settings(), calendar, services.events.control.clone());
    let screenshots = ScreenshotsBridge { home };
    follow(
        rt,
        settings(),
        screenshots,
        services.screenshots.control.clone(),
    );
}

#[cfg(test)]
mod tests {
    use super::start;
    use sill_services::env::Env;
    use sill_testkit::TempDir;

    /// `start` must never panic — the daemon has to reach the host connect and the
    /// IPC server — and every passive service holds its default (a service without its bus or
    /// compositor degrades to its own default, per `ServiceError`'s doc); the clock reads the
    /// time at once (`Thu 24 Sep 14:05`: four words).
    #[test]
    fn start_does_not_panic_and_passive_services_hold_their_defaults() {
        let rt = tokio::runtime::Runtime::new().unwrap_or_else(|e| panic!("{e}"));
        // No bus, no compositor, every directory in a scratch folder: nothing here reaches the
        // person's session or files.
        let scratch = TempDir::new("services-spawn");
        let (calls, _shell_calls) = tokio::sync::mpsc::unbounded_channel();
        let (summons, _launcher_effects) = tokio::sync::mpsc::unbounded_channel();
        let services = start(rt.handle(), &Env::isolated(scratch.path()), calls, summons);

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
        assert_eq!(
            *services.companion.state.borrow(),
            Default::default(),
            "no seat, no companion: nothing known"
        );
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
            *services.keycap.state.borrow(),
            sill_model::keycap::KeycapState::Unknown,
            "no socket and no system bus in an isolated Env: nothing is asked"
        );
        assert_eq!(
            services.intents.state.borrow().link,
            sill_model::intents::IntentsLink::Passive,
            "no bus name claimed until the daemon says to serve"
        );
        assert_eq!(
            services.idle.state.borrow().mode,
            sill_model::idle::IdleMode::Off(sill_model::idle::OffReason::NoPolicy),
            "no idle notification, display or logind touched until the daemon's policy"
        );
    }
}
