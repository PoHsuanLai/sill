use std::path::PathBuf;

use super::{
    Shot, ShotEffect, ShotEvent, ShotId, ShotKind, ShotLeave, ShotPreview, Shots, Taking, step,
};

fn shot(id: u64, name: &str, preview: ShotPreview) -> Shot {
    Shot {
        id: ShotId(id),
        path: PathBuf::from(format!("/tmp/shots/{name}")),
        preview,
    }
}

fn path(name: &str) -> PathBuf {
    PathBuf::from(format!("/tmp/shots/{name}"))
}

#[test]
fn a_screenshot_arrives_is_previewed_and_leaves() {
    let (one, fx) = step(Shots::default(), ShotEvent::Appeared(path("a.png")));
    assert_eq!(one.current, Some(shot(1, "a.png", ShotPreview::Pending)));
    assert_eq!(
        fx,
        vec![ShotEffect::MakePreview {
            id: ShotId(1),
            from: path("a.png")
        }]
    );
    let (again, fx) = step(one.clone(), ShotEvent::Appeared(path("a.png")));
    assert_eq!(
        (again, fx),
        (one.clone(), Vec::new()),
        "the same file twice"
    );
    let (ready, _) = step(one, ShotEvent::PreviewReady(ShotId(1), path("p.png")));
    assert_eq!(
        ready.current,
        Some(shot(1, "a.png", ShotPreview::Ready(path("p.png"))))
    );
    let cases = [
        (ShotLeave::Opened, vec![ShotEffect::Open(path("a.png"))]),
        (ShotLeave::Trashed, vec![ShotEffect::Trash(path("a.png"))]),
        (ShotLeave::Kept, Vec::new()),
        (ShotLeave::Dragged, Vec::new()),
        (ShotLeave::Moved, Vec::new()),
    ];
    for (why, want) in cases {
        let (gone, fx) = step(ready.clone(), ShotEvent::Left(ShotId(1), why));
        assert_eq!(gone.current, None, "{why:?}");
        assert_eq!(fx, want, "{why:?}");
    }
    let (stale, fx) = step(ready.clone(), ShotEvent::Left(ShotId(9), ShotLeave::Opened));
    assert_eq!((stale, fx), (ready, Vec::new()));
}

#[test]
fn a_newer_screenshot_replaces_the_current_one_and_stale_previews_drop() {
    let (one, _) = step(Shots::default(), ShotEvent::Appeared(path("a.png")));
    let (two, _) = step(one, ShotEvent::Appeared(path("b.png")));
    assert_eq!(two.current, Some(shot(2, "b.png", ShotPreview::Pending)));
    let (same, fx) = step(two.clone(), ShotEvent::PreviewReady(ShotId(1), path("p")));
    assert_eq!((same, fx), (two.clone(), Vec::new()));
    let (failed, _) = step(two, ShotEvent::PreviewFailed(ShotId(2)));
    assert_eq!(
        failed.current,
        Some(shot(2, "b.png", ShotPreview::Original))
    );
}

#[test]
fn one_tool_run_at_a_time() {
    let (running, fx) = step(Shots::default(), ShotEvent::Take(ShotKind::Area));
    assert_eq!(running.taking, Taking::Running(ShotKind::Area));
    assert_eq!(fx, vec![ShotEffect::RunTool(ShotKind::Area)]);
    let (still, fx) = step(running.clone(), ShotEvent::Take(ShotKind::Screen));
    assert_eq!((still, fx), (running.clone(), Vec::new()));
    let (idle, _) = step(running, ShotEvent::TakeEnded);
    assert_eq!(idle.taking, Taking::Idle);
}
