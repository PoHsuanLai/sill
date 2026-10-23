//! The dock's file on a fresh account: once the installed apps are known the dock starts with the
//! desktop's own, written once; a file that exists (even an emptied dock) is never seeded; a pin
//! made before the apps were known is not overwritten. Scratch directories only, no bus.

use std::path::PathBuf;
use std::time::Duration;

use sill_launcher::AppId;
use sill_model::apps::{
    App, AppIcon, AppIndex, AppsEvent, DbusActivation, Listing, Terminal, step,
};
use sill_model::pins::{DockPins, PinAt, PinEvent};
use sill_services::env::Env;
use sill_services::pins::PinsService;
use sill_services::service::{Handle, spawn};
use sill_testkit::TempDir;
use tokio::sync::watch;

const SETTLE: Duration = Duration::from_secs(5);
const QUIET: Duration = Duration::from_millis(300);

fn app(id: &str) -> App {
    App {
        id: AppId(id.to_owned()),
        name: id.to_owned(),
        generic_name: String::new(),
        comment: String::new(),
        exec: id.to_owned(),
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
    }
}

fn installed(ids: &[&str]) -> AppIndex {
    step(
        AppIndex::default(),
        AppsEvent::Scanned(ids.iter().map(|id| app(id)).collect()),
    )
}

fn ids(pins: &DockPins) -> Vec<&str> {
    pins.pinned.iter().map(|app| app.0.as_str()).collect()
}

fn start(scratch: &TempDir) -> (Handle<PinsService>, watch::Sender<AppIndex>, PathBuf) {
    let env = Env::isolated(scratch.path());
    let file = env.dirs.sill_config().join("layout.toml");
    let (apps, index) = watch::channel(AppIndex::default());
    let pins = spawn::<PinsService>(&tokio::runtime::Handle::current(), &env, index);
    (pins, apps, file)
}

async fn pinned_within(
    pins: &mut Handle<PinsService>,
    timeout: Duration,
    want: impl Fn(&DockPins) -> bool,
) -> bool {
    tokio::time::timeout(timeout, pins.state.wait_for(|state| want(state)))
        .await
        .is_ok_and(|seen| seen.is_ok())
}

#[tokio::test(flavor = "multi_thread")]
async fn a_fresh_dock_starts_with_the_desktops_apps_once_they_are_installed() {
    let scratch = TempDir::new("pins-seed-fresh");
    let (mut pins, apps, file) = start(&scratch);
    assert!(
        !pinned_within(&mut pins, QUIET, |p| !p.pinned.is_empty()).await,
        "no apps known: nothing to pin"
    );
    assert!(!file.exists(), "nothing written for nothing");
    apps.send_replace(installed(&[
        "org.quire.Settings",
        "firefox",
        "mailo",
        "org.quire.Anyview",
    ]));
    assert!(pinned_within(&mut pins, SETTLE, |p| !p.pinned.is_empty()).await);
    assert_eq!(
        ids(&pins.state.borrow()),
        ["mailo", "org.quire.Anyview", "org.quire.Settings"]
    );
    assert!(
        file.exists(),
        "the seed is written, so it is never made again"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn an_emptied_dock_stays_empty() {
    let scratch = TempDir::new("pins-seed-emptied");
    let folder = scratch.path().join("config/sill");
    std::fs::create_dir_all(&folder).unwrap_or_else(|e| panic!("{e}"));
    std::fs::write(folder.join("layout.toml"), "version = 1\ndock = []\n")
        .unwrap_or_else(|e| panic!("{e}"));
    let (mut pins, apps, _) = start(&scratch);
    apps.send_replace(installed(&["mailo", "org.quire.Settings"]));
    assert!(
        !pinned_within(&mut pins, QUIET, |p| !p.pinned.is_empty()).await,
        "a dock the person emptied is not filled again"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_pin_made_before_the_apps_were_known_is_kept() {
    let scratch = TempDir::new("pins-seed-pinned-first");
    let (mut pins, apps, _) = start(&scratch);
    let _ = pins.control.send(PinEvent::Pin {
        app: AppId("firefox".to_owned()),
        at: PinAt::End,
    });
    assert!(pinned_within(&mut pins, SETTLE, |p| !p.pinned.is_empty()).await);
    apps.send_replace(installed(&["mailo", "org.quire.Settings"]));
    assert!(
        !pinned_within(&mut pins, QUIET, |p| ids(p) != ["firefox"]).await,
        "the person's pin stands alone"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn an_old_dock_file_is_carried_over_and_never_seeded_over() {
    let scratch = TempDir::new("pins-seed-migrated");
    let folder = scratch.path().join("config/sill");
    std::fs::create_dir_all(&folder).unwrap_or_else(|e| panic!("{e}"));
    let old = r#"{"pinned":["firefox"]}"#;
    std::fs::write(folder.join("dock.json"), old).unwrap_or_else(|e| panic!("{e}"));
    let (mut pins, apps, file) = start(&scratch);
    assert!(pinned_within(&mut pins, SETTLE, |p| ids(p) == ["firefox"]).await);
    apps.send_replace(installed(&["mailo", "org.quire.Settings"]));
    assert!(
        !pinned_within(&mut pins, QUIET, |p| ids(p) != ["firefox"]).await,
        "the carried-over dock is not filled with the desktop's apps"
    );
    assert!(file.exists(), "the dock now lives in layout.toml");
    assert_eq!(
        std::fs::read_to_string(folder.join("dock.json")).unwrap_or_default(),
        old,
        "the old file is left as it was"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn an_edit_of_layout_toml_reaches_the_dock() {
    let scratch = TempDir::new("pins-live-reload");
    // The folder is there before the shell starts, as on any login: the watch is on it.
    let folder = scratch.path().join("config/sill");
    std::fs::create_dir_all(&folder).unwrap_or_else(|e| panic!("{e}"));
    let (mut pins, _apps, file) = start(&scratch);
    // Let the first read and the watch settle before the edit.
    tokio::time::sleep(SETTLE / 4).await;
    std::fs::write(
        &file,
        "version = 1\n[[dock]]\nkind = \"quire.dock.app\"\napp = \"btop\"\n",
    )
    .unwrap_or_else(|e| panic!("{e}"));
    assert!(pinned_within(&mut pins, SETTLE, |p| ids(p) == ["btop"]).await);
}
