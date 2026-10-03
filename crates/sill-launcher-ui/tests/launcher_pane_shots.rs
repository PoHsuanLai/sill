//! Pictures of the launcher's preview pane with the viewer's pane in it, rendered headless (vello
//! CPU) to `target/tmp/launcher-pane-<kind>.png` for review: a PDF's first page, a Markdown start
//! and a picture. Review material only, never a gate (the rasteriser changes with Blitz
//! revisions); each test still checks that the pane drew what it was asked to. A picture is a
//! `TextureLayer`, which the CPU painter leaves blank: its placeholder is what shows here.

mod launcher_harness;

use std::path::PathBuf;
use std::time::Duration;

use ds_harness::{Backdrop, Driver, Query};
use launcher_harness::{Open, Press, Seen, await_off_thread, loaded, open};

/// Where the pictures go.
fn shots() -> PathBuf {
    let base = option_env!("CARGO_TARGET_TMPDIR").map_or_else(std::env::temp_dir, PathBuf::from);
    std::fs::create_dir_all(&base).expect("the pictures' folder"); // test-only
    base
}

/// The pane shown over `row_downs` presses of Down from the Top Hit, once its peek has landed
/// (`pane`: the kind the viewer's pane says it shows), as `name`.
fn shoot(name: &str, pane: &str, row_downs: usize) {
    let mut open: Open = open("m9");
    open.press(Press::QuickLook);
    open.harness.advance(Duration::from_millis(40));
    for _ in 0..row_downs {
        open.press(Press::Down);
        open.harness.advance(Duration::from_millis(40));
    }
    let selector = format!(".anyview-pane[data-kind={pane}]");
    let drawn = await_off_thread(&mut open.harness, |h| h.count(&selector) == 1 && loaded(h));
    assert_eq!(drawn, Seen::Once, "the {pane} pane");
    open.harness.advance(Duration::from_millis(400));
    let image = open
        .harness
        .render_over(Backdrop::Scheme)
        .expect("the pane paints"); // test-only
    let path = shots().join(format!("launcher-pane-{name}.png"));
    image.save(&path).expect("the picture is written"); // test-only
    eprintln!("wrote {}", path.display());
}

#[test]
fn a_pdfs_first_page_in_the_pane() {
    shoot("pdf", "pdf", 0);
}

#[test]
fn a_picture_in_the_pane() {
    shoot("picture", "raster", 1);
}

#[test]
fn a_markdown_start_in_the_pane() {
    shoot("markdown", "markdown", 2);
}
