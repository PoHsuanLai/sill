//! The launch task: one `LaunchCommand` at a time, D-Bus activation or a transient
//! `systemd-run --user --scope`, tracked through [`reduce_launches`] until the scope exits.

use tokio::runtime::Handle;
use tokio::sync::{mpsc, watch};

use crate::apps::{App, AppIndex, DbusActivation};
use crate::launch::model::{LaunchOutcome, LaunchedScope, Launches, ScopeUnit, reduce_launches};
use crate::launch::plan::{launch_argv, scope_unit};
use crate::launch::{LaunchCommand, LaunchRequest};
use sill_launcher::AppId;

use super::dbus;
use super::open::{Manager, user_manager};

/// Run until `commands` closes (the `Control` was dropped).
pub(super) async fn run(
    rt: Handle,
    apps: watch::Receiver<AppIndex>,
    tx: watch::Sender<Launches>,
    mut commands: mpsc::UnboundedReceiver<LaunchCommand>,
) {
    let terminal = super::terminal::detect();
    let mut nonce: u32 = 0;
    while let Some(command) = commands.recv().await {
        match command {
            LaunchCommand::Launch(request) => {
                nonce += 1;
                launch_one(&rt, &apps, &tx, &terminal, request, nonce).await;
            }
            LaunchCommand::ShowInFiles(app_id) => show_in_files(&apps, &app_id).await,
            LaunchCommand::ForceQuit(app_id) => force_quit(&tx, &app_id).await,
            LaunchCommand::Open(request) => super::open::open(request).await,
        }
    }
}

async fn launch_one(
    rt: &Handle,
    apps: &watch::Receiver<AppIndex>,
    tx: &watch::Sender<Launches>,
    terminal: &[String],
    request: LaunchRequest,
    nonce: u32,
) {
    let app_id = request.app.clone();
    let Some(app) = apps.borrow().get(&app_id).cloned() else {
        record(tx, failed(app_id, "not in the app index"));
        return;
    };
    if app.activation == DbusActivation::DBus {
        record(tx, activate_over_dbus(&app, &request, app_id).await);
        return;
    }
    spawn_scope(rt, tx, &app, terminal, &request, app_id, nonce).await;
}

async fn activate_over_dbus(app: &App, request: &LaunchRequest, app_id: AppId) -> LaunchOutcome {
    match zbus::Connection::session().await {
        Ok(connection) => match dbus::activate(&connection, app, request).await {
            Ok(()) => LaunchOutcome::Activated(app_id),
            Err(error) => failed(app_id, &error.to_string()),
        },
        Err(error) => failed(app_id, &error.to_string()),
    }
}

async fn spawn_scope(
    rt: &Handle,
    tx: &watch::Sender<Launches>,
    app: &App,
    terminal: &[String],
    request: &LaunchRequest,
    app_id: AppId,
    nonce: u32,
) {
    let unit = scope_unit(&app_id, nonce);
    let argv = match launch_argv(app, request, &unit, terminal) {
        Ok(argv) => argv,
        Err(error) => {
            record(tx, failed(app_id, &error.to_string()));
            return;
        }
    };
    match start(&argv, request.token.as_deref()) {
        Ok(child) => {
            record(
                tx,
                LaunchOutcome::Started(LaunchedScope {
                    app: app_id,
                    unit: unit.clone(),
                }),
            );
            rt.spawn(wait_for_exit(tx.clone(), unit, child));
        }
        Err(error) => record(tx, failed(app_id, &error.to_string())),
    }
}

/// Start `argv` (`systemd-run --user --scope ...`-wrapped); when `systemd-run` itself cannot be
/// started (no user systemd manager, or it is not on `PATH`: `ErrorKind::NotFound`), fall back
/// to the bare command in its own process group, detached from sill's own (plan "Launch":
/// "fallback process group"). Either way the returned `Child` *is* the app (`systemd-run
/// --scope` execs in place of itself), so [`wait_for_exit`] does not need to know which path
/// was taken.
fn start(argv: &[String], token: Option<&str>) -> std::io::Result<tokio::process::Child> {
    start_with(argv, token, user_manager())
}

/// [`start`] for a user manager in `manager`'s state: when it is unreachable (no
/// `$XDG_RUNTIME_DIR/systemd/private`, as under a scratch runtime folder) `systemd-run --user`
/// would only exit 1, so the bare command runs in its own process group straight away (FINDINGS
/// "M9 lane a", F666).
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
        .map(|i| argv[i + 1..].to_vec())
        .unwrap_or_else(|| argv.to_vec())
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

/// `systemd-run --scope` execs the target in place of itself, so the spawned child's exit is
/// the scope's exit.
async fn wait_for_exit(
    tx: watch::Sender<Launches>,
    unit: ScopeUnit,
    mut child: tokio::process::Child,
) {
    let _ = child.wait().await;
    record(&tx, LaunchOutcome::Exited(unit));
}

async fn show_in_files(apps: &watch::Receiver<AppIndex>, app_id: &AppId) {
    let Some(app) = apps.borrow().get(app_id).cloned() else {
        return;
    };
    if let Ok(connection) = zbus::Connection::session().await {
        let _ = dbus::show_in_files(&connection, &app).await;
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

fn failed(app: AppId, reason: &str) -> LaunchOutcome {
    LaunchOutcome::Failed {
        app,
        reason: reason.to_owned(),
    }
}

fn record(tx: &watch::Sender<Launches>, outcome: LaunchOutcome) {
    tx.send_modify(|launches| *launches = reduce_launches(std::mem::take(launches), outcome));
}

#[cfg(test)]
mod tests {
    use super::{Manager, bare_command, start, start_with};

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
        let mut child = start(&argv, None).unwrap_or_else(|e| panic!("{e}"));
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
