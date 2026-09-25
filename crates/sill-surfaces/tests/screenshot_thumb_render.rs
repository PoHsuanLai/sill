//! The screenshot thumbnail in a Blitz harness, on the wall clock and the fake services: a ready
//! shot slides in (a pending one waits for its preview) and holds `screenshot.thumbnail_hold_ms`,
//! then says it was kept and plays out; the pointer on it pauses the hold and leaving it gives the
//! rest; a click opens it; a swipe right keeps it; Delete trashes it; a drag left or up hands it
//! nowhere yet (G120) and it stays; a newer shot replaces it; `screenshot.thumbnail = "hide"` and
//! another output draw nothing.

mod screenshot_thumb_support;

use std::time::Duration;

use ds::{Point, Px};
use ds_settings::Ms;
use screenshot_thumb_support::{Done, Running, start};
use shell_host::OutputId;
use sill_services::{ShotCommand, ShotId, ShotLeave, ShotPreview};
use sill_settings::{ScreenshotSettings, ShellFile, ShotThumbnail};
use sill_surfaces::surfaces::banners::BannerOutput;

const HOLD_MS: u64 = 1000;

fn file() -> ShellFile {
    ShellFile {
        screenshot: ScreenshotSettings {
            thumbnail_hold_ms: Ms(HOLD_MS as u16),
            ..ScreenshotSettings::default()
        },
        ..ShellFile::default()
    }
}

fn leave(id: u64, why: ShotLeave) -> Vec<ShotCommand> {
    vec![ShotCommand::Leave(ShotId(id), why)]
}

fn moved(from: Point, dx: f32, dy: f32) -> Point {
    Point {
        x: Px(from.x.0 + dx),
        y: Px(from.y.0 + dy),
    }
}

/// A drag by (`dx`, `dy`) from `from` in four moves 30 ms apart.
fn drag(running: &mut Running, from: Point, dx: f32, dy: f32) {
    running.harness.pointer_down(from);
    for n in 1..=4u8 {
        running.harness.advance(Duration::from_millis(30));
        let k = f32::from(n) / 4.0;
        running.harness.pointer_move(moved(from, dx * k, dy * k));
    }
    running.harness.advance(Duration::from_millis(30));
    running.harness.pointer_up(moved(from, dx, dy));
}

/// The card is gone from the stack (the exit has played) within `limit`.
fn gone_within(running: &mut Running, limit: Duration) -> Done {
    running.until(limit, |r| r.cards() == 0)
}

#[test]
fn a_shot_slides_in_holds_and_is_kept() {
    let mut running = start(file());
    running.harness.advance(Duration::from_millis(60));
    assert_eq!(running.cards(), 0, "nothing before a shot");
    running.shot(1, ShotPreview::Pending);
    running.harness.advance(Duration::from_millis(100));
    assert_eq!(running.cards(), 0, "a pending preview is waited for");
    running.ready(1);
    running.harness.advance(Duration::from_millis(20));
    assert_eq!(running.cards(), 1, "quire's stack slides it in");
    let src = running.harness.attr(".sill-thumb-image", "src");
    assert!(
        src.as_deref()
            .is_some_and(|s| s.starts_with("file://") && s.ends_with("Screenshot_fixture.png")),
        "{src:?}"
    );
    running
        .harness
        .advance(Duration::from_millis(HOLD_MS - 200));
    assert_eq!(running.cards(), 1, "held");
    assert_eq!(running.commands(), Vec::new());
    assert_eq!(
        running.until(Duration::from_millis(600), |r| r.leaving() == 1),
        Done::Yes,
        "the hold runs out"
    );
    assert_eq!(running.commands(), leave(1, ShotLeave::Kept));
    running.cleared();
    assert_eq!(
        gone_within(&mut running, Duration::from_millis(900)),
        Done::Yes
    );
}

#[test]
fn the_pointer_pauses_the_hold_and_leaving_gives_the_rest() {
    let mut running = start(file());
    running.one_up();
    let card = running.card();
    running.harness.pointer_move(card);
    running.harness.advance(Duration::from_millis(40));
    assert_eq!(
        running
            .harness
            .attr(".sill-thumb", "data-pointer")
            .as_deref(),
        Some("over")
    );
    running
        .harness
        .advance(Duration::from_millis(HOLD_MS + 500));
    assert_eq!((running.cards(), running.leaving()), (1, 0), "paused");
    assert_eq!(running.commands(), Vec::new());
    running.harness.pointer_move(Point {
        x: Px(2.0),
        y: Px(2.0),
    });
    let rest = u64::from(ShellFile::default().notifications.hover_min_remaining_ms.0);
    running.harness.advance(Duration::from_millis(rest - 300));
    assert_eq!(running.leaving(), 0, "the rest after a hover");
    assert_eq!(
        running.until(Duration::from_millis(900), |r| r.leaving() == 1),
        Done::Yes
    );
    assert_eq!(running.commands(), leave(1, ShotLeave::Kept));
}

#[test]
fn a_click_opens_it() {
    let mut running = start(file());
    running.one_up();
    let card = running.card();
    running.harness.click(card);
    running.harness.advance(Duration::from_millis(40));
    assert_eq!(running.commands(), leave(1, ShotLeave::Opened));
    assert_eq!(running.leaving(), 1);
}

#[test]
fn a_swipe_right_keeps_it_and_a_short_drag_springs_back() {
    let mut running = start(file());
    running.one_up();
    let card = running.card();
    drag(&mut running, card, 30.0, 0.0);
    running.harness.advance(Duration::from_millis(400));
    assert_eq!(running.commands(), Vec::new(), "a short drag");
    assert_eq!(running.card(), card, "springs back");
    drag(&mut running, card, 120.0, 0.0);
    running.harness.advance(Duration::from_millis(40));
    assert_eq!(running.commands(), leave(1, ShotLeave::Kept));
}

#[test]
fn delete_trashes_it() {
    let mut running = start(file());
    running.one_up();
    let card = running.card();
    running.harness.pointer_move(card);
    running.harness.advance(Duration::from_millis(300));
    let button = running
        .harness
        .centre(".sill-thumb-actions .ds-icon-button")
        .expect("the Delete button"); // test-only
    running.harness.pointer_move(button);
    running.harness.advance(Duration::from_millis(40));
    running.harness.click(button);
    running.harness.advance(Duration::from_millis(40));
    assert_eq!(running.commands(), leave(1, ShotLeave::Trashed));
}

#[test]
fn a_drag_out_has_nowhere_to_go_yet_and_the_card_stays() {
    let mut running = start(file());
    running.one_up();
    let card = running.card();
    running.harness.pointer_move(card);
    drag(&mut running, card, -100.0, -60.0);
    running.harness.advance(Duration::from_millis(400));
    assert_eq!(running.commands(), Vec::new());
    assert_eq!((running.cards(), running.leaving()), (1, 0));
}

#[test]
fn a_newer_shot_replaces_the_shown_one() {
    let mut running = start(file());
    running.one_up();
    running.ready(2);
    running.harness.advance(Duration::from_millis(60));
    assert_eq!(running.leaving(), 1, "the first plays out");
    assert_eq!(
        running.until(Duration::from_millis(900), |r| r.cards() == 1
            && r.leaving() == 0),
        Done::Yes
    );
    assert_eq!(
        running.commands(),
        Vec::new(),
        "a replaced shot's file stays"
    );
    running.harness.advance(Duration::from_millis(HOLD_MS));
    assert_eq!(running.commands(), leave(2, ShotLeave::Kept));
}

#[test]
fn hidden_by_the_setting_and_on_another_output() {
    let mut hidden = start(ShellFile {
        screenshot: ScreenshotSettings {
            thumbnail: ShotThumbnail::Hide,
            ..ScreenshotSettings::default()
        },
        ..ShellFile::default()
    });
    hidden.ready(1);
    hidden.harness.advance(Duration::from_millis(300));
    assert_eq!(hidden.cards(), 0, "screenshot.thumbnail = hide");

    let mut elsewhere = start(file());
    elsewhere.link.set(BannerOutput::On(OutputId(9)));
    elsewhere.ready(1);
    elsewhere.harness.advance(Duration::from_millis(300));
    assert_eq!(elsewhere.cards(), 0, "another output shows it");
}
