//! The wallpaper in a `ds_harness::Harness`, fed the frozen `wallpapers` channel: an entry for its
//! own output shows that entry's light-scheme path as a `file://` URL (the pinned appearance
//! resolves to the light scheme, `ds-settings`' own default), an entry for another output is
//! ignored, and no entry paints no image at all (`--paper` shows through instead). Painted over
//! nothing as a shell surface is, a known solid PNG covers the surface and the `--paper` ground
//! does with no image, so neither leaves the surface transparent (the live capture that showed the
//! compositor's grey through the wallpaper). The drawer setting decides whether there is a
//! surface at all.

use std::path::PathBuf;
use std::time::Duration;

use ds_harness::{Backdrop, Driver};
use sill_model::ids::OutputName;
use sill_model::wallpaper::{WallpaperImage, Wallpapers};
use sill_overlays::Wallpaper;
use sill_ui_kit::harness::SurfaceHarness;

/// The colour the fixture PNG is filled with.
const IMAGE: [u8; 3] = [220, 40, 20];

/// The surface under test: small, so vello_cpu paints it quickly.
const VIEWPORT: ds_harness::Viewport = ds_harness::Viewport {
    width: 320,
    height: 200,
    scale_percent: 100,
};

/// The wallpaper fed `fed`, drawn a few frames in.
fn run(fed: Wallpapers) -> SurfaceHarness<Wallpaper> {
    let mut run = SurfaceHarness::<Wallpaper>::setup((), ())
        .viewport(VIEWPORT)
        .start();
    run.feeds.wallpaper.send_replace(fed);
    run.harness.advance(Duration::from_millis(20));
    run
}

fn image(output: &str, light: &str, dark: &str) -> Wallpapers {
    Wallpapers {
        outputs: vec![WallpaperImage {
            output: OutputName(output.to_owned()),
            light: light.into(),
            dark: dark.into(),
        }],
    }
}

#[test]
fn no_entry_for_this_output_paints_no_image() {
    let html = run(Wallpapers::default()).harness.html();
    assert!(!html.contains("<img"), "{html}");
}

#[test]
fn an_entry_for_a_different_output_is_ignored() {
    let html = run(image(
        "HDMI-A-1",
        "/tmp/sill-wallpaper-render/other-light.png",
        "/tmp/sill-wallpaper-render/other-dark.png",
    ))
    .harness
    .html();
    assert!(!html.contains("<img"), "{html}");
}

#[test]
fn this_outputs_entry_shows_its_light_path_as_a_file_url() {
    let html = run(image(
        "DP-1",
        "/tmp/sill-wallpaper-render/light.png",
        "/tmp/sill-wallpaper-render/dark.png",
    ))
    .harness
    .html();
    assert!(
        html.contains("src=\"file:///tmp/sill-wallpaper-render/light.png\""),
        "{html}"
    );
    assert!(!html.contains("dark.png"), "{html}");
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
    let mut run = SurfaceHarness::<Wallpaper>::setup((), ())
        .viewport(VIEWPORT)
        .file_roots(shell_host::FileRoots::new([std::env::temp_dir()]))
        .start();
    run.feeds.wallpaper.send_replace(fed);
    let until = run.harness.now() + Duration::from_secs(2);
    loop {
        run.harness.advance(Duration::from_millis(20));
        let picture = run
            .harness
            .render_over(Backdrop::Clear)
            .expect("the wallpaper paints"); // test-only
        if done(&picture) || run.harness.now() >= until {
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
    let fed = image("DP-1", &path.to_string_lossy(), &path.to_string_lossy());
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
