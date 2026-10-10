//! Opening a file in the viewer, over a private `dbus-daemon` (`sill-testkit`), never the session
//! bus. The viewer is anyview's own `DbusInstance` owning `org.quire.Anyview1`, the real
//! implementation of the other end, so the call is checked against what the viewer parses: the
//! file, the place and the results the launcher chose it from arrive as the `Request` the
//! viewer's windows are opened from. Without a `dbus-daemon` they say so and skip.

use std::ffi::OsString;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use anyview_core::{FilePath, ResultsId, Resume, SequenceOrigin};
use anyview_platform::linux::DbusInstance;
use anyview_platform::{
    BusRoute as ViewerRoute, Claim, Env as ViewerEnv, Instance, Primary, Request,
};
use sill_launcher::{Generation, OpenTarget, ViewerOpen};
use sill_model::apps::AppIndex;
use sill_model::launch::{LaunchCommand, OpenRequest};
use sill_services::env::{BusRoute, Buses, Env, RecordedChildren};
use sill_services::launch::LaunchService;
use sill_services::service::spawn;
use sill_testkit::{PrivateBus, TempDir};
use tokio::sync::watch;

const WAIT: Duration = Duration::from_secs(5);

/// The viewer on `bus`: owning the name, ready to hear the launcher.
async fn viewer(bus: &PrivateBus, scratch: &TempDir) -> Primary {
    let mut env = ViewerEnv::isolated(scratch.path());
    env.session = ViewerRoute::Address(bus.address.clone());
    match DbusInstance::new(env)
        .claim(&Request::Open(Vec::new()))
        .await
        .unwrap_or_else(|e| panic!("{e}"))
    {
        Claim::Primary(primary) => primary,
        Claim::Forwarded => panic!("nobody else owns the viewer's name on a private bus"),
    }
}

/// The launch service on `bus`, told by the returned control, and the record of the children it
/// started (the isolated `Env` starts none: a fallback to `xdg-open` is recorded, never run).
fn launcher(
    bus: &PrivateBus,
    scratch: &TempDir,
) -> (sill_services::Control<LaunchCommand>, Arc<RecordedChildren>) {
    let children = RecordedChildren::shared();
    let mut env = Env::isolated(scratch.path());
    env.buses = Buses::new(BusRoute::Absent, BusRoute::Address(bus.address.clone()));
    env.children = children.clone();
    let (_apps, apps) = watch::channel(AppIndex::default());
    let control = spawn::<LaunchService>(&tokio::runtime::Handle::current(), &env, apps).control;
    (control, children)
}

/// The command lines `children` was asked to start, once there is one (or `WAIT` has gone).
async fn started_within(children: &RecordedChildren) -> Vec<Vec<OsString>> {
    let end = tokio::time::Instant::now() + WAIT;
    while children.started().is_empty() && tokio::time::Instant::now() < end {
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    children.started().into_iter().map(|s| s.argv).collect()
}

fn open(target: OpenTarget) -> LaunchCommand {
    LaunchCommand::Open(OpenRequest {
        target,
        token: None,
    })
}

fn file(text: &str) -> FilePath {
    FilePath::new(text).unwrap_or_else(|e| panic!("{e}"))
}

#[tokio::test(flavor = "multi_thread")]
async fn a_file_chosen_in_the_launcher_reaches_the_viewer_with_the_results_it_came_from() {
    let Some(bus) = PrivateBus::start() else {
        return;
    };
    let scratch = TempDir::new("launch-viewer-handoff");
    let mut viewer = viewer(&bus, &scratch).await;
    let (control, _children) = launcher(&bus, &scratch);

    control
        .send(open(OpenTarget::Viewer(ViewerOpen {
            file: PathBuf::from("/h/b.pdf"),
            search: Generation(41),
            results: ["/h/a.png", "/h/b.pdf", "/h/c.md"]
                .map(PathBuf::from)
                .to_vec(),
        })))
        .unwrap_or_else(|e| panic!("{e}"));

    let heard = viewer.next_within(WAIT);
    assert!(heard.is_some(), "the viewer heard nothing within {WAIT:?}");
    let Some(Request::Handoff(handoff)) = heard else {
        panic!("not a handoff: {heard:?}");
    };
    assert_eq!(handoff.file, file("/h/b.pdf"));
    assert_eq!(
        handoff.resume,
        Resume::Nothing,
        "the pane holds no place of its own"
    );
    let sequence = handoff
        .sequence
        .unwrap_or_else(|| panic!("no results arrived"));
    let names: Vec<&str> = sequence
        .entries()
        .iter()
        .map(|entry| entry.as_path().to_str().unwrap_or_default())
        .collect();
    assert_eq!(
        names,
        ["/h/a.png", "/h/b.pdf", "/h/c.md"],
        "in the order the launcher showed them"
    );
    assert_eq!(
        sequence.current(),
        &file("/h/b.pdf"),
        "pointing at the chosen file"
    );
    assert_eq!(sequence.origin(), &SequenceOrigin::Results(ResultsId(41)));
}

#[tokio::test(flavor = "multi_thread")]
async fn the_play_control_asks_the_viewer_to_play_the_file() {
    let Some(bus) = PrivateBus::start() else {
        return;
    };
    let scratch = TempDir::new("launch-viewer-play");
    let mut viewer = viewer(&bus, &scratch).await;
    let (control, _children) = launcher(&bus, &scratch);

    control
        .send(open(OpenTarget::Play(PathBuf::from("/m/song.flac"))))
        .unwrap_or_else(|e| panic!("{e}"));

    let heard = viewer.next_within(WAIT);
    assert!(heard.is_some(), "the viewer heard nothing within {WAIT:?}");
    assert_eq!(heard, Some(Request::Play(file("/m/song.flac"))));
}

#[tokio::test(flavor = "multi_thread")]
async fn without_a_viewer_on_the_bus_the_file_goes_to_the_default_handler() {
    let Some(bus) = PrivateBus::start() else {
        return;
    };
    let scratch = TempDir::new("launch-viewer-absent");
    let (control, children) = launcher(&bus, &scratch);

    control
        .send(open(OpenTarget::Play(PathBuf::from("/m/song.flac"))))
        .unwrap_or_else(|e| panic!("{e}"));

    assert_eq!(
        started_within(&children).await,
        [["xdg-open", "/m/song.flac"].map(OsString::from).to_vec()],
        "recorded, not run: an isolated `Env` starts no child"
    );
}
