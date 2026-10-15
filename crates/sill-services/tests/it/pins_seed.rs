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
    let file = env.dirs.sill_config().join("dock.json");
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
    std::fs::write(folder.join("dock.json"), r#"{"pinned":[]}"#).unwrap_or_else(|e| panic!("{e}"));
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
