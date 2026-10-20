use std::path::PathBuf;
use std::time::Duration;

use sill_launcher::{AppId, DesktopAction};
use sill_model::apps::{App, AppIcon, AppIndex, AppsEvent, DbusActivation, Listing, Terminal};
use sill_model::launch::{
    FailureCause, LaunchCommand, LaunchDone, LaunchRequest, Launches, answer_to,
};

use super::{LaunchLinks, LaunchService, monotonic_now};
use crate::Control;
use crate::NotificationsService;
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
        bus_names: Vec::new(),
        intents_app: None,
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
    let notifications = spawn::<NotificationsService>(&rt, &env, Default::default()).control;
    let launch = spawn::<LaunchService>(
        &rt,
        &env,
        LaunchLinks {
            apps: apps_rx,
            notifications,
        },
    );
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

/// What an isolated `Env` is asked to open: recorded, never started, so no test ever opens a file
/// in the person's session. With no session bus there is no viewer to take a file and no file
/// manager to reveal one, and with a scratch runtime folder no user systemd, so each target is
/// `xdg-open` on its own, and the recorder holds exactly those.
#[tokio::test]
async fn opening_with_no_session_records_the_opener_and_starts_nothing() {
    use std::ffi::OsString;

    use sill_launcher::{Generation, OpenTarget, ViewerOpen};
    use sill_model::launch::OpenRequest;

    use crate::env::{ProcessGroup, RecordedChildren};

    let (_apps_tx, apps_rx) = tokio::sync::watch::channel(AppIndex::default());
    let scratch = sill_testkit::TempDir::new("launch-open-recorded");
    let children = RecordedChildren::shared();
    let mut env = crate::env::Env::isolated(scratch.path());
    env.children = children.clone();
    let rt = tokio::runtime::Handle::current();
    let notifications = spawn::<NotificationsService>(&rt, &env, Default::default()).control;
    let links = LaunchLinks {
        apps: apps_rx,
        notifications,
    };
    let control = spawn::<LaunchService>(&rt, &env, links).control;

    let targets = [
        OpenTarget::File(PathBuf::from("/h/a.pdf")),
        OpenTarget::Viewer(ViewerOpen {
            file: PathBuf::from("/h/b.pdf"),
            search: Generation(1),
            results: vec![PathBuf::from("/h/b.pdf")],
        }),
        OpenTarget::Play(PathBuf::from("/m/song.flac")),
        OpenTarget::Url("https://example.com/".to_owned()),
    ];
    for target in &targets {
        control
            .send(LaunchCommand::Open(OpenRequest {
                target: target.clone(),
                token: Some("tok".to_owned()),
            }))
            .unwrap_or_else(|e| panic!("{e}"));
    }

    let end = tokio::time::Instant::now() + Duration::from_secs(5);
    while children.started().len() < targets.len() && tokio::time::Instant::now() < end {
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    let started = children.started();
    let opened: Vec<Vec<OsString>> = started.iter().map(|s| s.argv.clone()).collect();
    let want = [
        "/h/a.pdf",
        "/h/b.pdf",
        "/m/song.flac",
        "https://example.com/",
    ]
    .map(|what| vec![OsString::from("xdg-open"), OsString::from(what)]);
    assert_eq!(opened, want);
    assert!(
        started
            .iter()
            .all(|s| s.group == ProcessGroup::Own && s.token.as_deref() == Some("tok")),
        "bare, in its own process group, told the token"
    );
}

/// A launch service over a scratch runtime folder (so no user manager: the bare program runs in
/// its own process group) and the notifications service it tells.
fn start_over_scratch(
    index: AppIndex,
) -> (
    tokio::sync::watch::Receiver<Launches>,
    Control<LaunchCommand>,
    sill_testkit::TempDir,
) {
    let (_apps_tx, apps_rx) = tokio::sync::watch::channel(index);
    let rt = tokio::runtime::Handle::current();
    let scratch = sill_testkit::TempDir::new("launch-timing");
    let env = crate::env::Env::isolated(scratch.path());
    let notifications = spawn::<NotificationsService>(&rt, &env, Default::default()).control;
    let launch = spawn::<LaunchService>(
        &rt,
        &env,
        LaunchLinks {
            apps: apps_rx,
            notifications,
        },
    );
    (launch.state, launch.control, scratch)
}

fn ask(app: &str) -> LaunchCommand {
    LaunchCommand::Launch(LaunchRequest {
        app: AppId(app.to_owned()),
        action: DesktopAction::Default,
        files: Vec::new(),
        uris: Vec::new(),
        token: None,
    })
}

/// The end of a request for `app` made after `asked_at`, once the service has published it.
async fn end_of(
    state: &mut tokio::sync::watch::Receiver<Launches>,
    app: &str,
    asked_at: sill_model::launch::Stamp,
) -> LaunchDone {
    let app = AppId(app.to_owned());
    let found = tokio::time::timeout(
        Duration::from_secs(5),
        state.wait_for(|launches| answer_to(&launches.done, &app, asked_at).is_some()),
    )
    .await
    .unwrap_or_else(|_| panic!("no end within 5s"))
    .unwrap_or_else(|e| panic!("{e}"))
    .done
    .clone();
    answer_to(&found, &app, asked_at)
        .cloned()
        .unwrap_or_else(|| panic!("no end for {}", app.0))
}

#[tokio::test]
async fn a_launch_that_reaches_its_program_is_answered_with_its_timing() {
    let (mut state, control, _scratch) = start_over_scratch(index_with("sill-timed", "/bin/true"));
    let asked_at = monotonic_now();
    control
        .send(ask("sill-timed"))
        .unwrap_or_else(|e| panic!("{e}"));

    let LaunchDone::Spawned(timing) = end_of(&mut state, "sill-timed", asked_at).await else {
        panic!("the program did not start");
    };
    assert_eq!(timing.app, AppId("sill-timed".to_owned()));
    assert!(timing.requested_at >= asked_at);
    assert!(timing.spawned_at >= timing.requested_at);
    assert!(timing.pid.is_some(), "a program we started has a process");
    let unit = timing
        .unit
        .unwrap_or_else(|| panic!("a launch has a scope's name"));
    assert!(unit.0.starts_with("app-sill-sill\\x2dtimed-"), "{}", unit.0);
}

#[tokio::test]
async fn a_launch_of_an_app_that_is_not_installed_is_refused_with_its_cause() {
    let (mut state, control, _scratch) = start_over_scratch(AppIndex::default());
    let asked_at = monotonic_now();
    control
        .send(ask("no-such-app"))
        .unwrap_or_else(|e| panic!("{e}"));

    let LaunchDone::Refused {
        cause,
        requested_at,
        ..
    } = end_of(&mut state, "no-such-app", asked_at).await
    else {
        panic!("an app that is not there cannot start");
    };
    assert_eq!(cause, FailureCause::NotInstalled);
    assert!(requested_at >= asked_at);
}
