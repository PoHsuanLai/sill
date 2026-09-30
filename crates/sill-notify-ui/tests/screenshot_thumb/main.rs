//! The screenshot thumbnail in a Blitz harness, on the virtual clock and the fake services: a ready
//! shot slides in (a pending one waits for its preview) and holds `screenshot.thumbnail_hold_ms`,
//! then says it was kept and plays out; the pointer on it pauses the hold and leaving it gives the
//! rest; a click opens it; a swipe right keeps it; Delete trashes it; a drag left or up hands it
//! nowhere yet and it stays; a newer shot replaces it; `screenshot.thumbnail = "hide"` and
//! another output draw nothing; the keyboard ([`keys`]) is asked for and never taken.

use ds_harness::{Driver, Input, Query};
use std::path::PathBuf;
use std::time::Duration;

use ds::prelude::*;
use ds_settings::Ms;
use shell_host::OutputId;
use sill_model::screenshots::{Shot, ShotCommand, ShotId, ShotLeave, ShotPreview, Shots};
use sill_notify_ui::ScreenshotThumb;
use sill_settings::{ScreenshotSettings, ShellFile, ShotThumbnail};
use sill_ui_kit::harness::{SurfaceHarness, fake};
use sill_ui_kit::links::{BannerLink, BannerOutput};

mod keys;

type Running = SurfaceHarness<ScreenshotThumb>;

/// The services' own fixture screenshot, drawn as a ready preview.
const FIXTURE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../sill-services/tests/fixtures/shots/Screenshot_fixture.png"
);

/// Whether a wait ended in what it waited for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Done {
    Yes,
    No,
}

/// The thumbnail on `file`'s settings, shown on the test output, no shot yet.
fn start(file: ShellFile) -> Running {
    let link = BannerLink::new();
    link.set(BannerOutput::On(fake::output().id));
    SurfaceHarness::<ScreenshotThumb>::setup(link, ())
        .settings(file)
        .start()
}

/// Publish shot `id` as current with `preview`, taken at this instant (the next `advance` runs
/// the woken surface first).
fn shot(running: &mut Running, id: u64, preview: ShotPreview) {
    running.feeds.shots.send_replace(Shots {
        current: Some(Shot {
            id: ShotId(id),
            path: PathBuf::from(format!("/tmp/shots/Screenshot_{id}.png")),
            preview,
        }),
        last_id: ShotId(id),
        ..Shots::default()
    });
}

/// Publish shot `id` with the fixture as its ready preview.
fn ready(running: &mut Running, id: u64) {
    shot(running, id, ShotPreview::Ready(PathBuf::from(FIXTURE)));
}

/// The service dropping its current shot (it did, after a `Leave`).
fn cleared(running: &mut Running) {
    running.feeds.shots.send_replace(Shots::default());
}

/// Every command sent since the last call.
fn commands(running: &mut Running) -> Vec<ShotCommand> {
    std::iter::from_fn(|| running.feeds.shot_commands.try_recv().ok()).collect()
}

/// How many cards quire's stack draws (shown or playing out).
fn cards(running: &Running) -> usize {
    running.harness.count(".ds-shot")
}

/// How many cards (their own exit) or rows (after a swipe) are playing their exit.
fn leaving(running: &Running) -> usize {
    running.harness.count(".ds-shot[*|data-presence=leaving]")
        + running.harness.count(".ds-banner[*|data-presence=leaving]")
}

/// The card's centre.
fn card(running: &Running) -> Point {
    running.harness.centre(".ds-shot-picture").expect("a card") // test-only
}

/// Advance in 20 ms steps until `done`, at most `limit` of the harness's clock.
fn until(running: &mut Running, limit: Duration, done: impl Fn(&Running) -> bool) -> Done {
    let end = running.harness.now() + limit;
    while running.harness.now() < end {
        running.harness.advance(Duration::from_millis(20));
        if done(running) {
            return Done::Yes;
        }
    }
    Done::No
}

/// Shot 1, ready, and its entrance played.
fn one_up(running: &mut Running) {
    ready(running, 1);
    assert_eq!(
        until(running, Duration::from_millis(900), |r| cards(r) == 1),
        Done::Yes,
        "the card slides in"
    );
    running.harness.advance(Duration::from_millis(400));
}

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
    running.harness.send(Input::pointer_down(from));
    for n in 1..=4u8 {
        running.harness.advance(Duration::from_millis(30));
        let k = f32::from(n) / 4.0;
        running
            .harness
            .send(Input::pointer_move(moved(from, dx * k, dy * k)));
    }
    running.harness.advance(Duration::from_millis(30));
    running.harness.send(Input::pointer_up(moved(from, dx, dy)));
}

/// The card is gone from the stack (the exit has played) within `limit`.
fn gone_within(running: &mut Running, limit: Duration) -> Done {
    until(running, limit, |r| cards(r) == 0)
}

#[test]
fn a_shot_slides_in_holds_and_is_kept() {
    let mut running = start(file());
    running.harness.advance(Duration::from_millis(60));
    assert_eq!(cards(&running), 0, "nothing before a shot");
    shot(&mut running, 1, ShotPreview::Pending);
    running.harness.advance(Duration::from_millis(100));
    assert_eq!(cards(&running), 0, "a pending preview is waited for");
    ready(&mut running, 1);
    running.harness.advance(Duration::from_millis(20));
    assert_eq!(cards(&running), 1, "quire's stack slides it in");
    let src = running.harness.attr(".ds-shot-image", "src");
    assert!(
        src.as_deref()
            .is_some_and(|s| s.starts_with("file://") && s.ends_with("Screenshot_fixture.png")),
        "{src:?}"
    );
    // The hold starts as the ready shot shows, 20 ms ago.
    running
        .harness
        .advance(Duration::from_millis(HOLD_MS - 20 - 1));
    assert_eq!(
        (cards(&running), leaving(&running)),
        (1, 0),
        "held a millisecond before the hold's end"
    );
    assert_eq!(commands(&mut running), Vec::new());
    running.harness.advance(Duration::from_millis(1));
    assert_eq!(leaving(&running), 1, "the hold runs out at its end");
    assert_eq!(commands(&mut running), leave(1, ShotLeave::Kept));
    cleared(&mut running);
    assert_eq!(
        gone_within(&mut running, Duration::from_millis(900)),
        Done::Yes
    );
}

#[test]
fn the_pointer_pauses_the_hold_and_leaving_gives_the_rest() {
    let mut running = start(file());
    one_up(&mut running);
    let card = card(&running);
    running.harness.send(Input::pointer_move(card));
    running.harness.advance(Duration::from_millis(40));
    assert_eq!(
        running.harness.attr(".ds-shot", "data-hover").as_deref(),
        Some("on")
    );
    running
        .harness
        .advance(Duration::from_millis(HOLD_MS + 500));
    assert_eq!((cards(&running), leaving(&running)), (1, 0), "paused");
    assert_eq!(commands(&mut running), Vec::new());
    running.harness.send(Input::pointer_move(Point {
        x: Px(2.0),
        y: Px(2.0),
    }));
    let rest = u64::from(ShellFile::default().notifications.hover_min_remaining_ms.0);
    running.harness.advance(Duration::from_millis(rest - 300));
    assert_eq!(leaving(&running), 0, "the rest after a hover");
    assert_eq!(
        until(&mut running, Duration::from_millis(900), |r| leaving(r)
            == 1),
        Done::Yes
    );
    assert_eq!(commands(&mut running), leave(1, ShotLeave::Kept));
}

#[test]
fn a_click_opens_it() {
    let mut running = start(file());
    one_up(&mut running);
    let card = card(&running);
    running.harness.send(Input::click(card));
    running.harness.advance(Duration::from_millis(40));
    assert_eq!(commands(&mut running), leave(1, ShotLeave::Opened));
    assert_eq!(leaving(&running), 1);
}

#[test]
fn a_swipe_right_keeps_it_and_a_short_drag_springs_back() {
    let mut running = start(file());
    one_up(&mut running);
    let home = card(&running);
    drag(&mut running, home, 30.0, 0.0);
    running.harness.advance(Duration::from_millis(400));
    assert_eq!(commands(&mut running), Vec::new(), "a short drag");
    assert_eq!(card(&running), home, "springs back");
    drag(&mut running, home, 120.0, 0.0);
    running.harness.advance(Duration::from_millis(40));
    assert_eq!(commands(&mut running), leave(1, ShotLeave::Kept));
}

#[test]
fn delete_trashes_it() {
    let mut running = start(file());
    one_up(&mut running);
    let card = card(&running);
    running.harness.send(Input::pointer_move(card));
    running.harness.advance(Duration::from_millis(300));
    let button = running
        .harness
        .centre(".ds-shot-actions .ds-button")
        .expect("the Delete button"); // test-only
    running.harness.send(Input::pointer_move(button));
    running.harness.advance(Duration::from_millis(40));
    running.harness.send(Input::click(button));
    running.harness.advance(Duration::from_millis(40));
    assert_eq!(commands(&mut running), leave(1, ShotLeave::Trashed));
}

#[test]
fn a_drag_out_with_no_host_is_refused_the_card_stays_and_the_hold_resumes() {
    let mut running = start(file());
    one_up(&mut running);
    let card = card(&running);
    running.harness.send(Input::pointer_move(card));
    drag(&mut running, card, -100.0, -60.0);
    running.harness.advance(Duration::from_millis(400));
    assert_eq!(
        commands(&mut running),
        Vec::new(),
        "no drag source in a harness"
    );
    assert_eq!((cards(&running), leaving(&running)), (1, 0));
    // Refused, the hold gives the rest after a hover again and then keeps the file.
    let rest = u64::from(ShellFile::default().notifications.hover_min_remaining_ms.0);
    assert_eq!(
        until(
            &mut running,
            Duration::from_millis(rest + 900),
            |r| leaving(r) == 1
        ),
        Done::Yes
    );
    assert_eq!(commands(&mut running), leave(1, ShotLeave::Kept));
}

#[test]
fn the_picture_is_the_preview_file_at_its_own_shape() {
    let mut running = start(file());
    one_up(&mut running);
    let src = running.harness.attr(".ds-shot-image", "src");
    assert!(
        src.as_deref().is_some_and(|s| s.starts_with("file:///")),
        "{src:?}"
    );
    let plate = running
        .harness
        .rect("#thumb")
        .expect("the plate is the region");
    assert_eq!(plate.size.width.0.round(), 240.0);
    // Blitz loaded the file: the painted picture has the fixture's colour, not the card's.
    let image = running.harness.rect(".ds-shot-image").expect("the picture");
    let painted = running.harness.render().expect("a paint");
    let inside = |x: u32, y: u32| {
        let px = |v: f32| v.round() as u32;
        (px(image.origin.x.0) + 4..px(image.origin.x.0 + image.size.width.0) - 4).contains(&x)
            && (px(image.origin.y.0) + 4..px(image.origin.y.0 + image.size.height.0) - 4)
                .contains(&y)
    };
    let drawn = mean(
        painted
            .enumerate_pixels()
            .filter(|(x, y, _)| inside(*x, *y))
            .map(|(_, _, p)| p.0),
    );
    let fixture = image::open(FIXTURE).expect("the fixture").to_rgba8();
    let want = mean(fixture.pixels().map(|p| p.0));
    let apart = drawn
        .iter()
        .zip(want.iter())
        .map(|(a, b)| (a - b).abs())
        .fold(0.0_f64, f64::max);
    assert!(apart < 24.0, "painted {drawn:?}, the file {want:?}");
}

/// The mean red, green and blue of `pixels`.
fn mean(pixels: impl Iterator<Item = [u8; 4]>) -> [f64; 3] {
    let (sum, count) = pixels.fold(([0.0; 3], 0.0), |(mut sum, count), p| {
        for (s, v) in sum.iter_mut().zip(p) {
            *s += f64::from(v);
        }
        (sum, count + 1.0)
    });
    sum.map(|s| s / f64::max(count, 1.0))
}

#[test]
fn a_newer_shot_replaces_the_shown_one() {
    let mut running = start(file());
    one_up(&mut running);
    ready(&mut running, 2);
    running.harness.advance(Duration::from_millis(60));
    assert_eq!(leaving(&running), 1, "the first plays out");
    assert_eq!(
        until(&mut running, Duration::from_millis(900), |r| cards(r) == 1
            && leaving(r) == 0),
        Done::Yes
    );
    assert_eq!(
        commands(&mut running),
        Vec::new(),
        "a replaced shot's file stays"
    );
    running.harness.advance(Duration::from_millis(HOLD_MS));
    assert_eq!(commands(&mut running), leave(2, ShotLeave::Kept));
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
    ready(&mut hidden, 1);
    hidden.harness.advance(Duration::from_millis(300));
    assert_eq!(cards(&hidden), 0, "screenshot.thumbnail = hide");

    let mut elsewhere = start(file());
    elsewhere.link.set(BannerOutput::On(OutputId(9)));
    ready(&mut elsewhere, 1);
    elsewhere.harness.advance(Duration::from_millis(300));
    assert_eq!(cards(&elsewhere), 0, "another output shows it");
}
