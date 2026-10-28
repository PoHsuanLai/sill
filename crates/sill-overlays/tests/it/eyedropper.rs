//! The eyedropper on the harness, following an `EyedropperLink` the test writes as the daemon
//! does. With nothing asked it draws nothing; with a question and the picture of its output it
//! draws the still picture, and the magnifier once the pointer moves; a click answers with the
//! pixel under the pointer, Escape cancels, and either hides the surface at once.

use std::sync::Arc;
use std::time::Duration;

use ds::prelude::*;
use ds_harness::{Harness, Input, Viewport};
use image::{Rgba, RgbaImage};
use sill_model::capture_pick::{AskId, CaptureCommand, EyedropIn, Rgb8};
use sill_model::desktop::{OutputPicture, OutputShot};
use sill_overlays::ColorSampler;
use sill_ui_kit::harness::SurfaceHarness;
use sill_ui_kit::links::EyedropperLink;

type Running = SurfaceHarness<ColorSampler>;

/// The fake output is 1920 by 1080 logical pixels; the picture is a tenth of that, so one pixel of
/// the picture is ten logical pixels wide.
const VIEWPORT: Viewport = Viewport {
    width: 1920,
    height: 1080,
    scale_percent: 100,
};

/// A picture of 192 by 108 pixels whose pixel `(x, y)` is `(x, y, 7)`: every pixel names itself.
fn shot() -> OutputShot {
    let image = RgbaImage::from_fn(192, 108, |x, y| Rgba([x as u8, y as u8, 7, 255]));
    let mut png = std::io::Cursor::new(Vec::new());
    image
        .write_to(&mut png, image::ImageFormat::Png)
        .expect("a PNG"); // test-only
    OutputShot {
        size: (192, 108),
        rgba: Arc::from(image.into_raw()),
        png: Arc::from(png.into_inner()),
    }
}

fn start() -> Running {
    let mut running = SurfaceHarness::<ColorSampler>::setup(EyedropperLink::new(), ())
        .viewport(VIEWPORT)
        .start();
    running.harness.advance(Duration::from_millis(40));
    running
}

/// Question 1 asked of `DP-1` (the surface's output), the picture of `output` come.
fn asked_with_picture_of(output: &str) -> Running {
    let mut running = start();
    let _asked = running
        .link
        .take(EyedropIn::Asked(Some((AskId(1), vec!["DP-1".to_owned()]))));
    let _held = running.link.take(EyedropIn::Picture(
        output.to_owned(),
        OutputPicture::Shot(shot()),
    ));
    running.harness.advance(Duration::from_millis(200));
    running
}

fn at(x: f32, y: f32) -> Point {
    Point { x: Px(x), y: Px(y) }
}

fn answers(running: &mut Running) -> Vec<CaptureCommand> {
    std::iter::from_fn(|| running.feeds.capture_commands.try_recv().ok()).collect()
}

fn on_screen(harness: &Harness) -> usize {
    harness.count(".sill-eyedrop")
}

#[test]
fn it_exists_for_the_service_and_draws_nothing_until_a_picture_comes() {
    let running = start();
    assert_eq!(running.link.latest().surfaces, 1);
    assert_eq!(on_screen(&running.harness), 0);

    let mut asked = start();
    let _asked = asked
        .link
        .take(EyedropIn::Asked(Some((AskId(1), vec!["DP-1".to_owned()]))));
    asked.harness.advance(Duration::from_millis(100));
    assert_eq!(on_screen(&asked.harness), 0, "the picture has not come");

    let other = asked_with_picture_of("DP-2");
    assert_eq!(
        on_screen(&other.harness),
        0,
        "the picture is of another output"
    );
}

#[test]
fn a_picture_draws_the_still_and_the_magnifier_follows_the_pointer() {
    let mut running = asked_with_picture_of("DP-1");
    assert_eq!(on_screen(&running.harness), 1);
    assert_eq!(running.harness.count(".sill-eyedrop-still"), 1);
    assert_eq!(
        running.harness.count(".sill-eyedrop-loupe"),
        0,
        "no pointer yet"
    );

    running.harness.send(Input::pointer_move(at(105.0, 55.0)));
    running.harness.advance(Duration::from_millis(100));
    let html = running.harness.html();
    assert_eq!(running.harness.count(".sill-eyedrop-loupe"), 1);
    assert_eq!(
        running.harness.count(".sill-eyedrop-cell"),
        1,
        "the centre pixel is ringed"
    );
    assert!(
        html.contains("left:39px;top:-11px;width:132px;height:132px;"),
        "centred on the pointer: {html}"
    );
    assert!(
        html.contains("left:-60px;top:0px;width:2304px;height:1296px;"),
        "pixel (10, 5) lies in the middle cell: {html}"
    );

    running.harness.send(Input::pointer_move(at(305.0, 55.0)));
    running.harness.advance(Duration::from_millis(100));
    assert!(
        running.harness.html().contains("left:-300px;top:0px;"),
        "pixel (30, 5) now lies in the middle cell"
    );
}

#[test]
fn a_click_answers_with_the_pixel_under_the_pointer_and_hides_the_surface() {
    let mut running = asked_with_picture_of("DP-1");
    assert_eq!(
        answers(&mut running),
        vec![],
        "nothing is answered before the click"
    );
    running.harness.send(Input::click(at(105.0, 55.0)));
    running.harness.advance(Duration::from_millis(100));
    assert_eq!(
        answers(&mut running),
        vec![CaptureCommand::ColorAnswer(
            AskId(1),
            Some(Rgb8 {
                red: 10,
                green: 5,
                blue: 7
            })
        )]
    );
    assert_eq!(on_screen(&running.harness), 0, "hidden at the answer");
    running.harness.send(Input::click(at(405.0, 95.0)));
    assert_eq!(
        answers(&mut running),
        vec![],
        "a second click answers nothing"
    );
}

#[test]
fn escape_cancels_and_the_question_ending_takes_the_surface_away() {
    let mut running = asked_with_picture_of("DP-1");
    running.harness.send(Input::key(ShortcutKey::Escape));
    running.harness.advance(Duration::from_millis(100));
    assert_eq!(
        answers(&mut running),
        vec![CaptureCommand::ColorAnswer(AskId(1), None)]
    );
    assert_eq!(on_screen(&running.harness), 0);

    let mut ended = asked_with_picture_of("DP-1");
    assert_eq!(on_screen(&ended.harness), 1);
    let _gone = ended.link.take(EyedropIn::Asked(None));
    ended.harness.advance(Duration::from_millis(100));
    assert_eq!(on_screen(&ended.harness), 0);
    assert_eq!(
        answers(&mut ended),
        vec![],
        "the caller leaving answers nothing"
    );
}
