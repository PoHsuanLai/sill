//! `sill run`: take the single-instance lock, start the services and the IPC server, connect the
//! host, and keep one set of surfaces per output. The lock is
//! `latchkey::Agent::new("sill").listen()`; `AlreadyRunning` exits 0.
//!
//! `Links` is built once here and handed to the shell state, which gives it to every surface
//! root and to the IPC dispatch.

use std::process::ExitCode;
use std::time::{Duration, Instant};

use ds_settings::Store;
use shell_host::{Host, HostConfig, HostHandle};
use sill_model::locker::LockPhase;
use sill_services::env::{Dirs, Env};
use sill_settings::ShellFile;
use sill_ui_kit::links::{
    BannerLink, CenterLink, ControlCenterLink, CornerLink, EditWidgetsLink, EyedropperLink,
    ForceQuitLink, LauncherLink, Links, LockLink, OsdLink, PlacementFile, PowerMenuLink,
    SpacesBarLink, SplitPickerLink, SwitcherLink, WindowMenuLink,
};
use sill_ui_kit::services::Services;
use sill_ui_kit::settings::SettingsFeed;
use tokio::runtime::Runtime;
use tokio::sync::watch;

use super::events::{Shell, handle};
use super::msg::ShellMsg;
use super::{services, settings};
use crate::ipc::dispatch::quit::{Leaving, leaving_at};
use crate::ipc::server;
use crate::outputs::Outputs;

/// The most threads the blocking pool may hold. The services keep a few for good (volume, the DDC
/// worker) and the rest come and go with a scan or a lookup; a cap turns a runaway
/// `spawn_blocking` loop into queued work, not hundreds of threads (tokio's default is 512).
const BLOCKING_THREADS: usize = 48;

/// Run the daemon until `quit` or a fatal host error, on the seams `args` make.
pub fn run(args: &crate::run_args::RunArgs) -> ExitCode {
    super::hardening::forbid_core_files();
    let door = match take_door() {
        Ok(door) => door,
        Err(exit) => return exit,
    };
    let runtime = match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .max_blocking_threads(BLOCKING_THREADS)
        .build()
    {
        Ok(runtime) => runtime,
        Err(error) => return fatal(&format!("starting the async runtime: {error}")),
    };

    let env = args.env();
    let (shell_calls, shell_actions) = tokio::sync::mpsc::unbounded_channel();
    let (summons_tx, summons) = tokio::sync::mpsc::unbounded_channel();
    let (consents_tx, consents) = tokio::sync::mpsc::unbounded_channel();
    let services = services::start(runtime.handle(), &env, shell_calls, summons_tx, consents_tx);
    let store = settings::store(&env.dirs.config);
    let first = settings::load(&store);
    let (settings, settings_rx) = watch::channel(first.clone());
    let (user_style, user_style_rx) = watch::channel(settings::load_user_style(&store));
    let settings_feed = SettingsFeed {
        settings: settings_rx,
        user_style: user_style_rx,
    };
    let links = build_links(&runtime, &env, &settings_feed, &settings, &store);
    services::follow_settings(runtime.handle(), &settings_feed, &links, &services, &env);
    crate::voice::spawn_consent_writer(runtime.handle(), consents, store.clone(), settings.clone());

    // The desktop's own compositor hands the shell a connection; any other session is reached
    // through the environment.
    let handed = runtime.block_on(sill_services::desktop_wl::quire::shell_connection(
        &env.buses.session,
    ));
    let handed_by_compositor = handed.is_some();
    let route = super::compositor::host_route(handed);
    let host = match Host::<ShellMsg>::connect(host_config(&first, &env.dirs, route)) {
        Ok(host) => host,
        Err(error) => return fatal(&format!("connecting to the compositor: {error}")),
    };
    let host_handle = host.handle();
    // The person has entered the desktop: the first start offers the assistant's model picker.
    if let Err(error) =
        sill_services::model_offer::offer_first_run(&env.dirs, &services.launch.control)
    {
        eprintln!("sill: offering the model picker: {error}");
    }
    let seat = match handed_by_compositor {
        true => super::compositor::ShellSeat::linked(
            runtime.handle().clone(),
            host_handle.clone(),
            services.dock_rects.subscribe(),
        ),
        false => super::compositor::ShellSeat::absent(),
    };
    // Detached: it accepts connections until the process exits with the host (doc on
    // `ipc::server::spawn`).
    let _ipc_thread = server::spawn(door, host_handle.clone());

    crate::release::spawn_confirmation(
        runtime.handle(),
        env.buses.session.route(),
        host_handle.clone(),
        env.children.clone(),
    );
    spawn_followers(
        &runtime,
        &services,
        &settings_feed,
        &host_handle,
        &links,
        &store,
    );
    crate::intents::spawn_follower(runtime.handle(), host_handle.clone(), shell_actions);
    crate::companion::spawn_follower(
        runtime.handle(),
        host_handle.clone(),
        summons,
        &links.launcher,
        services.companion.control.clone(),
    );
    let locker = services.locker.state.clone();
    let mut shell = Shell {
        outputs: Outputs::default(),
        shell_lock: crate::lock::ShellLock::default(),
        services,
        settings_feed,
        store,
        settings,
        user_style,
        links,
        keys: sill_model::keyboard::ShellKeys::default(),
        seat,
    };
    let result = host.run(move |ctx, event| handle(&mut shell, ctx, event));

    shutdown_runtime(runtime);

    let phase = locker.borrow().phase;
    match result {
        Ok(()) => exit_clean_at(phase),
        Err(error) => fatal(&format!("{error}")),
    }
}

/// The status of a host that ended on its own (not an error): a session that is locked or being
/// locked is still covered, and the compositor keeps it covered with nobody to take a password,
/// so the exit is a failure and the unit starts the shell again, which locks at once.
fn exit_clean_at(phase: LockPhase) -> ExitCode {
    match leaving_at(phase) {
        Leaving::Allowed => ExitCode::SUCCESS,
        Leaving::Refused => {
            eprintln!("sill: leaving while the session is locked; the shell is restarted");
            ExitCode::FAILURE
        }
    }
}

/// The single-instance lock: another daemon already running exits 0.
fn take_door() -> Result<latchkey::Listening, ExitCode> {
    let agent = latchkey::Agent::new("sill")
        .map_err(|error| fatal(&format!("naming this agent: {error}")))?;
    agent.listen().map_err(|error| match error {
        latchkey::Error::AlreadyRunning => ExitCode::SUCCESS,
        error => fatal(&format!("taking the single-instance lock: {error}")),
    })
}

/// The host's configuration: shell-host's, with the first-mouse window from `first`'s settings
/// and the connection `route` names.
fn host_config(first: &ShellFile, dirs: &Dirs, route: shell_host::WaylandRoute) -> HostConfig {
    HostConfig {
        first_mouse_window_ms: shell_host::Ms(u32::from(first.menus.first_mouse_window_ms.0)),
        route,
        ..sill_ui_kit::root::host_config(dirs)
    }
}

/// Build the shared links every surface root is given.
fn build_links(
    runtime: &Runtime,
    env: &Env,
    settings_feed: &SettingsFeed,
    settings: &watch::Sender<ShellFile>,
    store: &Store,
) -> Links {
    let places = PlacementFile::in_dirs(&env.dirs);
    Links {
        focus: settings::spawn_focus(
            runtime.handle(),
            settings_feed.settings.clone(),
            store.clone(),
        ),
        launcher: LauncherLink::new(),
        osd: OsdLink::new(),
        power_menu: PowerMenuLink::new(),
        control_center: ControlCenterLink::new(),
        notification_center: CenterLink::new(),
        banners: BannerLink::new(),
        corners: CornerLink::new(),
        switcher: SwitcherLink::new(),
        lock: LockLink::new(),
        window_menu: WindowMenuLink::new(),
        split_picker: SplitPickerLink::new(),
        force_quit: ForceQuitLink::new(),
        spaces_bar: SpacesBarLink::new(),
        eyedropper: EyedropperLink::new(),
        edit_widgets: EditWidgetsLink::new()
            .with_places(places.clone())
            .with_writer(crate::edit_widgets::writer(
                store.clone(),
                places,
                settings.clone(),
            )),
    }
}

/// The tasks that act on the host from a service or a setting: the own lock screen's wish to hold
/// the session lock, the idle dim surfaces (mapped while the idle service dims), the settings
/// file's watch, and the hot corners' firings, carried out off the host thread.
fn spawn_followers(
    runtime: &Runtime,
    services: &Services,
    settings_feed: &SettingsFeed,
    host: &HostHandle<ShellMsg>,
    links: &Links,
    store: &Store,
) {
    crate::lock::spawn_follower(runtime.handle(), services, host.clone());
    crate::idle_dim::spawn_follower(runtime.handle(), services, host.clone());
    settings::spawn_watch(runtime, store, host.clone());
    settings::spawn_user_style_watch(runtime, store, host.clone());
    crate::window_menu::spawn_picks(runtime.handle(), links, host.clone());
    crate::split_picker::spawn_answers(runtime.handle(), links, host.clone());
    crate::force_quit::spawn_asks(runtime.handle(), links, host.clone());
    crate::spaces_bar::spawn_picks(runtime.handle(), links, host.clone());
    crate::spaces_bar::spawn_outbox(runtime.handle(), links, host.clone());
    crate::eyedropper::spawn_follower(runtime.handle(), links, services, host.clone());
    crate::corners::spawn_hits(
        runtime.handle(),
        links.clone(),
        settings_feed.settings.clone(),
        services.clone(),
        host.clone(),
    );
}

/// The bound [`shutdown_runtime`] gives every service's blocking-pool thread to finish once
/// `host.run` has returned (`ctx.exit()`; every surface, and with it every clone of `Services`,
/// is already gone by then — shell-host's own `Host::run` clears every surface before
/// returning). Comfortably above `sill-services`' own bounded waits (a few
/// seconds at most), but not unbounded.
const RUNTIME_SHUTDOWN_BOUND: Duration = Duration::from_secs(4);

/// Ends `runtime`, waiting at most [`RUNTIME_SHUTDOWN_BOUND`] for every spawned task's thread to
/// finish. Dropping a `tokio::runtime::Runtime` plainly waits forever for that, and a
/// `spawn_blocking` task that calls into something that hangs (an output tool under a
/// compositor without `zwlr_output_manager_v1`) would otherwise keep
/// the daemon from ever exiting after `sill quit` already got its answer. Past the bound, that
/// thread (and whatever it is stuck in) is left running and the process exits without it; a
/// warning on stderr says so.
fn shutdown_runtime(runtime: tokio::runtime::Runtime) {
    shutdown_runtime_within(runtime, RUNTIME_SHUTDOWN_BOUND);
}

/// [`shutdown_runtime`]'s body, `bound` taken as a parameter so a test can shrink it rather than
/// waiting out the real one.
fn shutdown_runtime_within(runtime: tokio::runtime::Runtime, bound: Duration) {
    let started = Instant::now();
    runtime.shutdown_timeout(bound);
    if started.elapsed() >= bound {
        eprintln!(
            "sill: the async runtime did not shut down within {bound:?}; a service's thread is \
             still running and is left behind as the process exits"
        );
    }
}

fn fatal(message: &str) -> ExitCode {
    eprintln!("sill: {message}");
    ExitCode::FAILURE
}

#[cfg(test)]
mod shutdown_tests {
    use std::sync::mpsc;
    use std::time::{Duration, Instant};

    use super::shutdown_runtime_within;

    /// A `spawn_blocking` task that never returns — the shape of an output tool hanging under a
    /// compositor without `zwlr_output_manager_v1` — does not keep
    /// [`shutdown_runtime_within`] waiting past its bound; the leaked thread is left running
    /// (past the bound `forever` is never drained again, so it stays parked) and the call still
    /// returns.
    #[test]
    fn a_stuck_blocking_task_does_not_hang_the_bound() {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()
            .unwrap_or_else(|e| panic!("{e}"));
        let (never_sent, forever) = mpsc::channel::<()>();
        runtime.spawn_blocking(move || {
            // `never_sent` moves in and is held here (not dropped) for as long as this closure
            // runs, so `forever.recv()` below blocks rather than seeing a disconnected sender.
            let _held = never_sent;
            let _ = forever.recv();
        });
        let bound = Duration::from_millis(200);
        let started = Instant::now();
        shutdown_runtime_within(runtime, bound);
        assert!(
            started.elapsed() < Duration::from_secs(5),
            "shutdown waited well past its {bound:?} bound"
        );
    }

    /// A runtime with nothing left running shuts down well inside its bound: the bound is a
    /// ceiling, not a fixed wait.
    #[test]
    fn a_clean_runtime_shuts_down_promptly() {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()
            .unwrap_or_else(|e| panic!("{e}"));
        let bound = Duration::from_secs(2);
        let started = Instant::now();
        shutdown_runtime_within(runtime, bound);
        assert!(
            started.elapsed() < bound,
            "a clean shutdown should not need the whole bound"
        );
    }
}
