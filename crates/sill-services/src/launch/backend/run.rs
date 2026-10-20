//! The launch task: one `LaunchCommand` at a time, D-Bus activation or a transient
//! `systemd-run --user --scope`, tracked through the launch state's step until the scope exits.

use sill_launcher::AppId;
use sill_model::apps::{App, AppIndex, DbusActivation};
use sill_model::launch::{
    FailureCause, LaunchCommand, LaunchOutcome, LaunchRequest, LaunchTiming, LaunchedScope,
    Launches, Pid, ScopeUnit, SettingsOpen, Stamp, launch_argv, scope_unit,
};
use std::time::{Duration, Instant};

use tokio::runtime::Handle;
use tokio::sync::{mpsc, watch};

use super::LaunchLinks;
use super::clock::monotonic_now;
use super::notice::Tell;
use super::open::{Manager, user_manager};
use super::outcomes::{Outcomes, failed};
use super::{Reach, dbus};
use crate::deadline::{BUS_CALL, LAUNCH_COMMAND, call, within};
use crate::env::{Bus, ConnectError};

/// Run until `commands` closes (the `Control` was dropped).
pub(crate) async fn run(
    setup: Reach,
    links: LaunchLinks,
    tx: watch::Sender<Launches>,
    mut commands: mpsc::UnboundedReceiver<LaunchCommand>,
) {
    let rt = Handle::current();
    let LaunchLinks {
        apps,
        notifications,
    } = links;
    let tell = Tell(notifications);
    let terminal = super::terminal::wrapper(&setup.terminal);
    let mut nonce: u32 = 0;
    while let Some(command) = commands.recv().await {
        // Stamped as the request reaches the launch code, before anything is awaited.
        let requested_at = monotonic_now();
        let what = match &command {
            LaunchCommand::Launch(_) => "launching an app",
            LaunchCommand::ShowInFiles(_) => "showing an app in Files",
            LaunchCommand::ForceQuit(_) => "force quitting an app",
            LaunchCommand::Open(_) => "opening an item",
            LaunchCommand::OpenSettings(_) => "opening Settings",
        };
        let handled = within(
            LAUNCH_COMMAND,
            what,
            handle(
                &rt,
                &setup,
                &outcomes_for(&tx, &tell, &apps),
                &terminal,
                &mut nonce,
                command,
                requested_at,
            ),
        )
        .await;
        if let Err(error) = handled {
            eprintln!("sill: launch: {error}");
        }
    }
}

/// What the outcomes of a command are recorded into.
fn outcomes_for(
    tx: &watch::Sender<Launches>,
    tell: &Tell,
    apps: &watch::Receiver<AppIndex>,
) -> Outcomes {
    Outcomes {
        tx: tx.clone(),
        tell: tell.clone(),
        apps: apps.clone(),
    }
}

/// Carry out one command.
async fn handle(
    rt: &Handle,
    setup: &Reach,
    outcomes: &Outcomes,
    terminal: &[String],
    nonce: &mut u32,
    command: LaunchCommand,
    requested_at: Stamp,
) {
    match command {
        LaunchCommand::Launch(request) => {
            *nonce += 1;
            let attempt = Attempt {
                nonce: *nonce,
                requested_at,
            };
            launch_one(rt, setup, outcomes, terminal, request, attempt).await;
        }
        LaunchCommand::ShowInFiles(app_id) => {
            show_in_files(&setup.session, &outcomes.apps, &app_id).await
        }
        LaunchCommand::ForceQuit(app_id) => force_quit(&outcomes.tx, &app_id).await,
        LaunchCommand::Open(request) => super::open::open(setup, request).await,
        LaunchCommand::OpenSettings(open) => open_settings(&setup.session, &open).await,
    }
}

/// One launch request's place in the service: its scope's number, and when it arrived.
#[derive(Debug, Clone, Copy)]
struct Attempt {
    nonce: u32,
    requested_at: Stamp,
}

async fn launch_one(
    rt: &Handle,
    setup: &Reach,
    outcomes: &Outcomes,
    terminal: &[String],
    request: LaunchRequest,
    attempt: Attempt,
) {
    let app_id = request.app.clone();
    let Some(app) = outcomes.apps.borrow().get(&app_id).cloned() else {
        outcomes.refused(
            app_id,
            attempt.requested_at,
            FailureCause::NotInstalled,
            "not in the app index",
        );
        return;
    };
    if app.activation == DbusActivation::DBus {
        let activated = activate_over_dbus(&setup.session, &app, &request).await;
        match activated {
            Ok(()) => {
                outcomes.record(LaunchOutcome::Activated(app_id.clone()));
                outcomes.spawned(LaunchTiming {
                    app: app_id,
                    unit: None,
                    requested_at: attempt.requested_at,
                    spawned_at: monotonic_now(),
                    pid: None,
                });
            }
            Err(no) => outcomes.refused(app_id, attempt.requested_at, no.cause, &no.detail),
        }
        return;
    }
    let manager = user_manager(setup.runtime.as_deref());
    spawn_scope(rt, outcomes, &app, terminal, &request, attempt, manager).await;
}

/// Why an activation did not happen.
struct Refusal {
    cause: FailureCause,
    detail: String,
}

async fn activate_over_dbus(
    session: &Bus,
    app: &App,
    request: &LaunchRequest,
) -> Result<(), Refusal> {
    let refusal = |cause, detail: String| Refusal { cause, detail };
    match session.try_connect().await {
        Ok(connection) => call(
            BUS_CALL,
            "the app",
            dbus::activate(&connection, app, request),
        )
        .await
        .map_err(|error| refusal(FailureCause::NoAnswer, error.to_string())),
        Err(error) => Err(refusal(FailureCause::NoStarter, error.to_string())),
    }
}

async fn spawn_scope(
    rt: &Handle,
    outcomes: &Outcomes,
    app: &App,
    terminal: &[String],
    request: &LaunchRequest,
    attempt: Attempt,
    manager: Manager,
) {
    let app_id = request.app.clone();
    let Attempt {
        nonce,
        requested_at,
    } = attempt;
    let unit = match scope_unit(&app_id, nonce) {
        Ok(unit) => unit,
        Err(error) => {
            let detail = error.to_string();
            outcomes.refused(app_id, requested_at, FailureCause::CannotRun, &detail);
            return;
        }
    };
    let argv = match launch_argv(app, request, &unit, terminal) {
        Ok(argv) => argv,
        Err(error) => {
            let detail = error.to_string();
            outcomes.refused(app_id, requested_at, FailureCause::CannotRun, &detail);
            return;
        }
    };
    match start_with(&argv, request.token.as_deref(), manager) {
        Ok(child) => {
            // The child exists: `systemd-run --scope` becomes the app in place, so its pid is the app's.
            let spawned_at = monotonic_now();
            let pid = child.id().map(Pid);
            outcomes.record(LaunchOutcome::Started(LaunchedScope {
                app: app_id.clone(),
                unit: unit.clone(),
            }));
            outcomes.spawned(LaunchTiming {
                app: app_id.clone(),
                unit: Some(unit.clone()),
                requested_at,
                spawned_at,
                pid,
            });
            rt.spawn(wait_for_exit(outcomes.clone(), app_id, unit, child));
        }
        Err(error) => {
            let detail = error.to_string();
            outcomes.refused(app_id, requested_at, FailureCause::CannotRun, &detail);
        }
    }
}

/// Start `argv` (`systemd-run --user --scope ...`-wrapped) for a user manager in `manager`'s
/// state. When it is unreachable (no `$XDG_RUNTIME_DIR/systemd/private`, as under a scratch
/// runtime folder) `systemd-run --user` would only exit 1, so the bare command runs in its own
/// process group straight away; when `systemd-run` itself cannot be started (not on `PATH`:
/// `ErrorKind::NotFound`) the bare command runs too. Either way the returned `Child` *is* the
/// app (`systemd-run --scope` execs in place of itself), so [`wait_for_exit`] does not need to
/// know which path was taken.
fn start_with(
    argv: &[String],
    token: Option<&str>,
    manager: Manager,
) -> std::io::Result<tokio::process::Child> {
    if manager == Manager::Missing {
        return spawn_argv(&bare_command(argv), token, true);
    }
    match spawn_argv(argv, token, false) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            spawn_argv(&bare_command(argv), token, true)
        }
        result => result,
    }
}

/// `argv` after `systemd-run`'s own `--`: the terminal-wrapped `Exec`, with no scope wrapper.
fn bare_command(argv: &[String]) -> Vec<String> {
    argv.iter()
        .position(|arg| arg == "--")
        .map_or_else(|| argv.to_vec(), |i| argv[i + 1..].to_vec())
}

/// `argv[0]` with `argv[1..]`, the activation token (if any) in both `XDG_ACTIVATION_TOKEN`
/// (xdg-activation) and `DESKTOP_STARTUP_ID` (the older startup-notification convention some
/// toolkits still read); `own_process_group` starts it as the leader of a fresh process group
/// (the fallback path only — the systemd scope path needs none, systemd already isolates it).
fn spawn_argv(
    argv: &[String],
    token: Option<&str>,
    own_process_group: bool,
) -> std::io::Result<tokio::process::Child> {
    let (program, args) = argv.split_first().ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::InvalidInput, "empty argument vector")
    })?;
    let mut command = tokio::process::Command::new(program);
    command.args(args);
    if let Some(token) = token {
        command.env("XDG_ACTIVATION_TOKEN", token);
        command.env("DESKTOP_STARTUP_ID", token);
    }
    command.stdin(std::process::Stdio::null());
    command.kill_on_drop(false);
    if own_process_group {
        command.process_group(0);
    }
    command.spawn()
}

/// An app that ends with an error this soon after it started did not really open.
const QUICK_EXIT: Duration = Duration::from_secs(10);

/// `systemd-run --scope` execs the target in place of itself, so the spawned child's exit is
/// the scope's exit. An exit with an error code within [`QUICK_EXIT`] is a failed launch (a
/// signal, as Force Quit sends, is not).
async fn wait_for_exit(
    outcomes: Outcomes,
    app: AppId,
    unit: ScopeUnit,
    mut child: tokio::process::Child,
) {
    let started = Instant::now();
    let status = child.wait().await;
    let errored = status
        .ok()
        .and_then(|status| status.code())
        .is_some_and(|code| code != 0);
    if errored && started.elapsed() < QUICK_EXIT {
        outcomes.record(failed(
            app,
            FailureCause::ClosedAtOnce,
            "exited with an error at once",
        ));
    }
    outcomes.record(LaunchOutcome::Exited(unit));
}

async fn show_in_files(session: &Bus, apps: &watch::Receiver<AppIndex>, app_id: &AppId) {
    let Some(app) = apps.borrow().get(app_id).cloned() else {
        return;
    };
    if let Ok(connection) = session.try_connect().await {
        let _ = call(
            BUS_CALL,
            "the file manager",
            dbus::show_in_files(&connection, &app),
        )
        .await;
    }
}

/// `systemctl --user kill --signal=KILL` every `app-sill-*` scope this service started for
/// `app_id`; each scope's own [`wait_for_exit`] task reports the `Exited` outcome when it dies,
/// so this sends the signal only.
async fn force_quit(tx: &watch::Sender<Launches>, app_id: &AppId) {
    let units: Vec<ScopeUnit> = tx
        .borrow()
        .scopes
        .iter()
        .filter(|scope| &scope.app == app_id)
        .map(|scope| scope.unit.clone())
        .collect();
    for unit in units {
        let _ = tokio::process::Command::new("systemctl")
            .args(["--user", "kill", "--signal=KILL", &unit.0])
            .status()
            .await;
    }
}

/// Settings at the place `open` names, started by the bus when it is not running.
async fn open_settings(session: &Bus, open: &SettingsOpen) {
    if let Some(connection) = session_connection(session).await
        && let Err(error) = call(BUS_CALL, "Settings", dbus::open_settings(&connection, open)).await
    {
        eprintln!("sill: launch: opening Settings: {error}");
    }
}

/// The session bus, or none when there is not one (logged when it could not be reached).
async fn session_connection(session: &Bus) -> Option<zbus::Connection> {
    match session.try_connect().await {
        Ok(connection) => Some(connection),
        Err(ConnectError::NoBus(_)) => None,
        Err(error) => {
            eprintln!("sill: launch: opening Settings: {error}");
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Manager, bare_command, start_with};

    #[test]
    fn bare_command_takes_what_follows_the_scopes_own_dash_dash() {
        let wrapped = vec![
            "systemd-run".to_owned(),
            "--user".to_owned(),
            "--scope".to_owned(),
            "--".to_owned(),
            "ghostty".to_owned(),
            "-e".to_owned(),
            "htop".to_owned(),
        ];
        assert_eq!(
            bare_command(&wrapped),
            vec!["ghostty".to_owned(), "-e".to_owned(), "htop".to_owned()]
        );
        let no_wrapper = vec!["/bin/true".to_owned()];
        assert_eq!(bare_command(&no_wrapper), no_wrapper);
    }

    /// `start` never needs a real `systemd-run` to be exercised: a made-up `argv[0]` triggers
    /// the same `ErrorKind::NotFound` a missing user systemd would, and the fallback runs the
    /// bare command (the part after `--`) directly.
    #[tokio::test]
    async fn a_program_that_cannot_start_falls_back_to_the_bare_command() {
        let argv = vec![
            "sill-launch-test-no-such-binary-anywhere".to_owned(),
            "--".to_owned(),
            "/bin/true".to_owned(),
        ];
        let mut child =
            start_with(&argv, None, Manager::Reachable).unwrap_or_else(|e| panic!("{e}"));
        let status = child.wait().await.unwrap_or_else(|e| panic!("{e}"));
        assert!(status.success());
    }

    /// With no reachable user manager the wrapper is never run: `/bin/false` stands where
    /// `systemd-run` would be, so only the bare command after `--` can succeed.
    #[tokio::test]
    async fn an_unreachable_user_manager_runs_the_bare_command() {
        let argv: Vec<String> = ["/bin/false", "--user", "--scope", "--", "/bin/true"]
            .iter()
            .map(|s| (*s).to_owned())
            .collect();
        let cases = [(Manager::Missing, true), (Manager::Reachable, false)];
        for (manager, succeeds) in cases {
            let mut child = start_with(&argv, None, manager).unwrap_or_else(|e| panic!("{e}"));
            let status = child.wait().await.unwrap_or_else(|e| panic!("{e}"));
            assert_eq!(status.success(), succeeds, "{manager:?}");
        }
    }
}
