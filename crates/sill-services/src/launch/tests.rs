use std::path::PathBuf;
use std::time::Duration;

use sill_launcher::{AppId, DesktopAction};
use sill_model::apps::{App, AppIcon, AppIndex, AppsEvent, DbusActivation, Listing, Terminal};
use sill_model::launch::{LaunchCommand, LaunchRequest};

use super::LaunchService;
use crate::service::spawn;

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
        categories: Vec::new(),
        keyboard_profile: None,
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
