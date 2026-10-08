//! The launch task: one `LaunchCommand` at a time, D-Bus activation or a transient
//! `systemd-run --user --scope`, tracked through [`step`] until the scope exits.

use sill_launcher::AppId;
use sill_model::apps::{App, AppIndex, DbusActivation};
use sill_model::launch::{
    LaunchCommand, LaunchOutcome, LaunchRequest, LaunchedScope, Launches, SETTINGS_BUS_NAME,
    ScopeUnit, SettingsAccount, launch_argv, scope_unit, step,
};
use tokio::runtime::Handle;
use tokio::sync::{mpsc, watch};

use super::open::{Manager, user_manager};
use super::{Reach, dbus};
use crate::env::Bus;

/// Run until `commands` closes (the `Control` was dropped).
pub(crate) async fn run(
    setup: Reach,
    apps: watch::Receiver<AppIndex>,
    tx: watch::Sender<Launches>,
    mut commands: mpsc::UnboundedReceiver<LaunchCommand>,
) {
    let rt = Handle::current();
    let terminal = super::terminal::detect();
    let mut nonce: u32 = 0;
    while let Some(command) = commands.recv().await {
        match command {
            LaunchCommand::Launch(request) => {
                nonce += 1;
                launch_one(&rt, &setup, &apps, &tx, &terminal, request, nonce).await;
            }
            LaunchCommand::ShowInFiles(app_id) => {
                show_in_files(&setup.session, &apps, &app_id).await
            }
            LaunchCommand::ForceQuit(app_id) => force_quit(&tx, &app_id).await,
            LaunchCommand::Open(request) => super::open::open(&setup, request).await,
            LaunchCommand::OpenSettings(account) => open_settings(&setup.session, &account).await,
        }
    }
}

async fn launch_one(
    rt: &Handle,
    setup: &Reach,
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
    if app.activation == DbusActivation::DBus || app.id.0 == SETTINGS_BUS_NAME {
        record(
            tx,
            activate_over_dbus(&setup.session, &app, &request, app_id).await,
        );
        return;
    }
    let manager = user_manager(setup.runtime.as_deref());
    spawn_scope(rt, tx, &app, terminal, &request, nonce, manager).await;
}

async fn activate_over_dbus(
    session: &Bus,
    app: &App,
    request: &LaunchRequest,
    app_id: AppId,
) -> LaunchOutcome {
    match session.connect().await {
        Ok(Some(connection)) => match dbus::activate(&connection, app, request).await {
            Ok(()) => LaunchOutcome::Activated(app_id),
            Err(error) => failed(app_id, &error.to_string()),
        },
        Ok(None) => failed(app_id, "no session bus"),
        Err(error) => failed(app_id, &error.to_string()),
    }
}

async fn spawn_scope(
    rt: &Handle,
    tx: &watch::Sender<Launches>,
    app: &App,
    terminal: &[String],
    request: &LaunchRequest,
    nonce: u32,
    manager: Manager,
) {
    let app_id = request.app.clone();
    let unit = scope_unit(&app_id, nonce);
    let argv = match launch_argv(app, request, &unit, terminal) {
        Ok(argv) => argv,
        Err(error) => {
            record(tx, failed(app_id, &error.to_string()));
            return;
        }
    };
    match start_with(&argv, request.token.as_deref(), manager) {
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

async fn show_in_files(session: &Bus, apps: &watch::Receiver<AppIndex>, app_id: &AppId) {
    let Some(app) = apps.borrow().get(app_id).cloned() else {
        return;
    };
    if let Ok(Some(connection)) = session.connect().await {
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
    tx.send_modify(|launches| *launches = step(std::mem::take(launches), outcome));
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

/// Settings at `account`, started by the bus when it is not running.
async fn open_settings(session: &Bus, account: &SettingsAccount) {
    match session.connect().await {
        Ok(Some(connection)) => {
            if let Err(error) = dbus::open_settings(&connection, account).await {
                eprintln!("sill: launch: opening Settings: {error}");
            }
        }
        Ok(None) => {}
        Err(error) => eprintln!("sill: launch: opening Settings: {error}"),
    }
}
