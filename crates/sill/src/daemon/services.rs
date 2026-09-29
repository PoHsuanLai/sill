//! Starting every service, each after the ones in its `Deps`, into the [`Services`] every
//! surface reads.

use std::sync::Arc;

use sill_search::results::{ResultsLinks, ResultsService, Sources};
use sill_services::apps::AppsService;
use sill_services::auth::AuthService;
use sill_services::battery::BatteryService;
use sill_services::clipboard::ClipboardService;
use sill_services::cosmic_wl::CosmicWlService;
use sill_services::env::Env;
use sill_services::idle::{IdleLinks, IdleService};
use sill_services::launch::LaunchService;
use sill_services::locker::LockerService;
use sill_services::service::spawn;
use sill_services::volume::VolumeService;
use sill_surfaces::services::Services;
use tokio::runtime::Handle as Runtime;

/// Start every service on `rt` (the `cosmic_wl` and volume threads on their own threads)
/// against `env`'s seams, each after the ones it depends on. A service whose system half is
/// missing still yields its receiver, at its default.
pub(crate) fn start(rt: &Runtime, env: &Env) -> Services {
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
        let services = start(rt.handle(), &Env::isolated(scratch.path()));

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
