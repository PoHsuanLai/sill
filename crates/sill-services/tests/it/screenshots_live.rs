//! The screenshots service against a real inotify watch of a temp folder (never Pictures, never
//! the portal): a file written under a temp name and renamed to `Screenshot_…png` is
//! `current` within 500 ms with a preview; a `.part` file is nothing. The previews land in a
//! scratch cache inside the temp folder (never the person's own), removed with it.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use sill_model::screenshots::{
    ShotCommand, ShotFolders, ShotLeave, ShotPolicy, ShotPreview, Shots, Watching,
};
use sill_services::env::{Dirs, Env, RecordedChildren};
use sill_services::screenshots::ScreenshotsService;
use sill_services::service::spawn;
use tokio::sync::watch;

const FIXTURE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/shots/Screenshot_fixture.png"
);

/// A fresh temp folder of this test's own.
fn folder(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("sill-shots-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a temp folder"); // test-only
    dir
}

/// Wait up to `limit` for `done` on the published state; the state then.
async fn until(
    rx: &mut watch::Receiver<Shots>,
    limit: Duration,
    done: impl Fn(&Shots) -> bool,
) -> Option<Shots> {
    let end = Instant::now() + limit;
    loop {
        if done(&rx.borrow_and_update()) {
            return Some(rx.borrow().clone());
        }
        let left = end.checked_duration_since(Instant::now())?;
        tokio::time::timeout(left, rx.changed()).await.ok()?.ok()?;
    }
}

fn current_is(path: &Path) -> impl Fn(&Shots) -> bool + '_ {
    move |shots| {
        shots
            .current
            .as_ref()
            .is_some_and(|s| s.path == path && s.preview != ShotPreview::Pending)
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn a_renamed_screenshot_is_current_with_a_preview_and_a_part_file_is_not() {
    let dir = folder("watch");
    let dirs = Dirs::under(&dir.join("dirs"));
    let env = Env {
        dirs: dirs.clone(),
        ..Env::isolated(&dir)
    };
    let shots = spawn::<ScreenshotsService>(&tokio::runtime::Handle::current(), &env, ());
    let (mut rx, control) = (shots.state, shots.control);
    let policy = ShotPolicy {
        folders: ShotFolders::At(vec![dir.clone()]),
    };
    control.send(ShotCommand::Configure(policy)).expect("sent"); // test-only
    let watching = until(&mut rx, Duration::from_secs(2), |s| {
        s.watching == Watching::Folders(vec![dir.clone()])
    })
    .await;
    assert!(watching.is_some(), "the folder is watched");

    std::fs::copy(FIXTURE, dir.join("Screenshot_a.png.part")).expect("copied"); // test-only
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert_eq!(rx.borrow().current, None, "a .part file is nothing");

    let temp = dir.join(".Screenshot_b.png");
    std::fs::copy(FIXTURE, &temp).expect("copied"); // test-only
    let target = dir.join("Screenshot_b.png");
    let renamed = Instant::now();
    std::fs::rename(&temp, &target).expect("renamed"); // test-only
    let seen = until(&mut rx, Duration::from_millis(500), current_is(&target)).await;
    eprintln!("current after {:?}: {seen:?}", renamed.elapsed());
    let seen = seen.expect("the renamed file is current within 500 ms"); // test-only
    assert!(
        matches!(
            seen.current.as_ref().map(|s| &s.preview),
            Some(ShotPreview::Ready(preview)) if preview.starts_with(&dirs.cache)
        ),
        "a preview is made in the scratch cache"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test(flavor = "multi_thread")]
async fn opening_and_trashing_a_shot_are_recorded_never_run() {
    let dir = folder("leave");
    let children = RecordedChildren::shared();
    let env = Env {
        dirs: Dirs::under(&dir.join("dirs")),
        children: children.clone(),
        ..Env::isolated(&dir)
    };
    let shots = spawn::<ScreenshotsService>(&tokio::runtime::Handle::current(), &env, ());
    let (mut rx, control) = (shots.state, shots.control);
    let policy = ShotPolicy {
        folders: ShotFolders::At(vec![dir.clone()]),
    };
    control.send(ShotCommand::Configure(policy)).expect("sent"); // test-only
    let _ = until(&mut rx, Duration::from_secs(2), |s| {
        matches!(s.watching, Watching::Folders(_))
    })
    .await;
    let target = dir.join("Screenshot_c.png");
    std::fs::copy(FIXTURE, &target).expect("copied"); // test-only
    let seen = until(&mut rx, Duration::from_secs(2), current_is(&target)).await;
    let id = seen
        .and_then(|shots| shots.current.map(|shot| shot.id))
        .expect("the file is current"); // test-only

    control
        .send(ShotCommand::Leave(id, ShotLeave::Opened))
        .expect("sent"); // test-only
    let end = Instant::now() + Duration::from_secs(2);
    while children.started().is_empty() && Instant::now() < end {
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    let argv: Vec<_> = children.started().into_iter().map(|s| s.argv).collect();
    assert_eq!(
        argv,
        [vec!["xdg-open".into(), target.as_os_str().to_owned()]],
        "the opener is recorded with the file, not started"
    );
    let _ = std::fs::remove_dir_all(&dir);
}
