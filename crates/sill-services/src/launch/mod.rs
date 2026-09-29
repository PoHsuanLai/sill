//! Starting apps (activation token, DBusActivatable via org.freedesktop.Application.Activate,
//! Terminal=true wrapper, `systemd-run --user --scope --collect --unit=app-sill-<id>-<rand>.scope`
//! (fallback process group)).
//!
//! The activation token comes from shell-host (`SurfaceHandle::activation_token`) in the
//! daemon's `sill::launch`, which then sends a [`LaunchCommand`] here. [`LaunchService`] prefers
//! `org.freedesktop.Application.Activate` for a `DBusActivatable` entry (`dbus.rs`); otherwise
//! it plans the argument vector (`plan.rs`, pure) and runs it inside a transient
//! `systemd-run --user --scope`, wrapped in the machine's terminal (`terminal.rs`) when the
//! entry asks for one, tracking the scope until it exits (`run.rs`).

mod dbus;
mod model;
mod open;
mod plan;
mod run;
mod terminal;

pub use model::{LaunchOutcome, LaunchedScope, Launches, ScopeUnit, step};
pub use open::{OpenRequest, open_argv, reveal_folder};
pub use plan::{launch_argv, scope_unit};

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use sill_launcher::{AppId, DesktopAction};
use tokio::sync::watch;

use crate::env::{Bus, Env};
use crate::error::ServiceError;
use crate::service::{Service, ServiceIo};
use sill_model::apps::AppIndex;

/// One launch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LaunchRequest {
    pub app: AppId,
    pub action: DesktopAction,
    /// Dropped files (`%f`/`%F`).
    pub files: Vec<PathBuf>,
    /// Dropped URIs (`%u`/`%U`).
    pub uris: Vec<String>,
    /// The xdg-activation token for `XDG_ACTIVATION_TOKEN` / `platform-data`; `None` when the
    /// compositor refused one.
    pub token: Option<String>,
}

/// What the launch service can be told.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum LaunchCommand {
    Launch(LaunchRequest),
    /// `org.freedesktop.FileManager1.ShowItems([desktop file uri])` (dock Cmd-click).
    ShowInFiles(AppId),
    /// SIGKILL the app's `app-sill-*` scopes (dock Force Quit).
    ForceQuit(AppId),
    /// Open a file, reveal it, or open an address with the default handler (the launcher's rows).
    Open(OpenRequest),
}

/// What the launch service reaches: D-Bus activation and the file manager on `session`, the
/// user's systemd manager under `runtime` (`$XDG_RUNTIME_DIR`).
#[derive(Debug, Clone, PartialEq, Eq)]
struct Reach {
    session: Bus,
    runtime: Option<PathBuf>,
}

impl Reach {
    /// `env`'s session bus and runtime directory.
    fn of(env: &Env) -> Reach {
        Reach {
            session: env.buses.session.clone(),
            runtime: env.dirs.runtime.clone(),
        }
    }
}

/// App launching as a [`Service`]: scopes, activation tokens, opening files. Its `Deps` is the
/// app index, which resolves ids to entries.
#[derive(Debug)]
pub enum LaunchService {}

impl Service for LaunchService {
    const NAME: &'static str = "launch";
    type State = Launches;
    type Command = LaunchCommand;
    type Deps = watch::Receiver<AppIndex>;

    fn run(
        env: &Env,
        deps: Self::Deps,
        io: ServiceIo<Self>,
    ) -> impl Future<Output = Result<(), ServiceError>> + Send + 'static {
        let reach = Reach::of(env);
        async move {
            run::run(reach, deps, io.state, io.commands).await;
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{LaunchCommand, LaunchRequest, LaunchService};
    use crate::service::spawn;
    use sill_launcher::{AppId, DesktopAction};
    use sill_model::apps::{App, AppIcon, AppIndex, AppsEvent, DbusActivation, Listing, Terminal};
    use std::path::PathBuf;
    use std::time::Duration;

    fn index_with(id: &str, exec: &str) -> AppIndex {
        let app = App {
            id: AppId(id.to_owned()),
            name: id.to_owned(),
            generic_name: String::new(),
            comment: String::new(),
            exec: exec.to_owned(),
            icon: AppIcon::None,
            startup_wm_class: None,
            flatpak: None,
            terminal: Terminal::No,
            activation: DbusActivation::Exec,
            actions: Vec::new(),
            mime_types: Vec::new(),
            keywords: Vec::new(),
            listing: Listing::Shown,
            path: PathBuf::from(format!("/usr/share/applications/{id}.desktop")),
        };
        sill_model::apps::step(AppIndex::default(), AppsEvent::Scanned(vec![app]))
    }

    /// Exercises the real `systemd-run --user --scope` path on this machine: `/bin/true` starts,
    /// `Launches` reports `Started` with a well-formed unit, and once the process exits the same
    /// unit is reported `Exited`. Run explicitly (`cargo test -p sill-services -- --ignored
    /// systemd_scope`); needs a user systemd (`systemctl --user status`).
    #[tokio::test]
    #[ignore = "needs a user systemd"]
    async fn a_launch_runs_in_a_real_systemd_scope_and_exits() {
        let (_apps_tx, apps_rx) =
            tokio::sync::watch::channel(index_with("sill-launch-test", "/bin/true"));
        let rt = tokio::runtime::Handle::current();
        let scratch = sill_testkit::TempDir::new("launch-scope");
        let mut env = crate::env::Env::isolated(scratch.path());
        env.dirs.runtime = std::env::var_os("XDG_RUNTIME_DIR").map(std::path::PathBuf::from);
        let launch = spawn::<LaunchService>(&rt, &env, apps_rx);
        let (mut launches, control) = (launch.state, launch.control);

        control
            .send(LaunchCommand::Launch(LaunchRequest {
                app: AppId("sill-launch-test".to_owned()),
                action: DesktopAction::Default,
                files: Vec::new(),
                uris: Vec::new(),
                token: None,
            }))
            .unwrap_or_else(|e| panic!("{e}"));

        let started = tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                launches.changed().await.unwrap_or_else(|e| panic!("{e}"));
                if let Some(scope) = launches.borrow().scopes.first() {
                    return scope.unit.clone();
                }
            }
        })
        .await
        .unwrap_or_else(|_| panic!("no Started outcome within 5s"));
        eprintln!("sill: live launch started unit {}", started.0);
        assert!(started.0.starts_with("app-sill-"));
        assert!(started.0.ends_with(".scope"));

        tokio::time::timeout(Duration::from_secs(10), async {
            loop {
                if launches.borrow().scopes.is_empty() {
                    return;
                }
                launches.changed().await.unwrap_or_else(|e| panic!("{e}"));
            }
        })
        .await
        .unwrap_or_else(|_| panic!("the scope never reported Exited within 10s"));
        eprintln!("sill: live launch unit {} exited", started.0);
    }
}
