//! The wallpaper's picture in a `ds_native::Harness`, painted over nothing as a shell surface
//! is: a known solid PNG fed as this output's light image must cover the surface's centre, and
//! with no image the `--paper` ground must, so neither leaves the surface transparent (the live
//! capture that showed the compositor's grey through the wallpaper).

mod support;

use std::cell::RefCell;
use std::path::PathBuf;
use std::time::Duration;

use dioxus::prelude::*;
use ds_native::{Backdrop, Clock, Harness, HarnessConfig, Viewport};
use sill_model::ids::OutputName;
use sill_services::{Services, WallpaperImage, Wallpapers};
use sill_surfaces::Wallpaper;
use tokio::sync::watch;

/// The colour the fixture PNG is filled with.
const IMAGE: [u8; 3] = [220, 40, 20];

/// The surface under test: small, so vello_cpu paints it quickly.
const VIEWPORT: Viewport = Viewport {
    width: 320,
    height: 200,
    scale_percent: 100,
};

thread_local! {
    static SERVICES: RefCell<Option<Services>> = const { RefCell::new(None) };
}

/// The root: the fake services, then the wallpaper on the fake output.
fn app() -> Element {
    let services = SERVICES.with(|s| s.borrow().clone());
    let Some(services) = services else {
        return rsx! {};
    };
    use_context_provider(|| services);
    rsx! {
        Wallpaper { output: support::fake::output() }
    }
}

/// A solid `IMAGE` PNG in the temp directory, named for `tag` so tests do not share one.
fn fixture(tag: &str) -> PathBuf {
    let [r, g, b] = IMAGE;
    let path = std::env::temp_dir().join(format!(
        "sill-wallpaper-paint-{tag}-{}.png",
        std::process::id()
    ));
    image::RgbaImage::from_pixel(64, 40, image::Rgba([r, g, b, 255]))
        .save(&path)
        .expect("the fixture PNG writes"); // test-only
    path
}

/// The wallpaper fed `fed`, advanced until `done` holds (at most two seconds), then painted
/// over a transparent backdrop.
fn paint(fed: Wallpapers, done: impl Fn(&image::RgbaImage) -> bool) -> image::RgbaImage {
    let (mut services, _feeds) = support::fake::services();
    let (_tx, rx) = watch::channel(fed);
    services.wallpaper.state = rx;
    SERVICES.with(|s| *s.borrow_mut() = Some(services));
    let mut harness =
        Harness::with_config(app, HarnessConfig::new(VIEWPORT).with_clock(Clock::Virtual));
    let until = harness.now() + Duration::from_secs(2);
    loop {
        harness.advance(Duration::from_millis(20));
        let picture = harness
            .render_over(Backdrop::Clear)
            .expect("the wallpaper paints"); // test-only
        if done(&picture) || harness.now() >= until {
            return picture;
        }
    }
}

fn centre(picture: &image::RgbaImage) -> [u8; 4] {
    picture
        .get_pixel(picture.width() / 2, picture.height() / 2)
        .0
}

fn is_image(pixel: [u8; 4]) -> bool {
    let [r, g, b] = IMAGE;
    pixel == [r, g, b, 255]
}

#[test]
fn this_outputs_image_covers_the_surface() {
    let path = fixture("image");
    let fed = Wallpapers {
        outputs: vec![WallpaperImage {
            output: OutputName("DP-1".to_owned()),
            light: path.clone(),
            dark: path,
        }],
    };
    let picture = paint(fed, |p| is_image(centre(p)));
    assert!(is_image(centre(&picture)), "centre {:?}", centre(&picture));
    let (right, bottom) = (picture.width() - 3, picture.height() - 3);
    for (x, y) in [(2, 2), (right, bottom)] {
        let corner = picture.get_pixel(x, y).0;
        assert!(is_image(corner), "corner ({x}, {y}) {corner:?}");
    }
}

#[test]
fn no_image_paints_an_opaque_paper_ground() {
    let picture = paint(Wallpapers::default(), |p| centre(p)[3] == 255);
    assert_eq!(centre(&picture)[3], 255, "centre {:?}", centre(&picture));
}
