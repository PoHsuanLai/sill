//! Every surface root reads the host's signals the same way: a key turns the focus ring on
//! (`data-modality="keyboard"`), a press turns it off, and the host's scale reaches the root's
//! pixel tokens. Driven through `SurfaceHarness`, whose own seat stands in for shell-host's (the
//! bridge that republishes shell-host's is tested in `root/signals_tests.rs`). The surfaces here
//! are the ones that once drew their `Ds` outside the bridge.

use std::time::Duration;

use ds::{PixelToken, Scale, ShortcutKey};
use ds_harness::{Driver, Input, Query, Viewport};
use sill_bar::Bar;
use sill_dock::Dock;
use sill_overlays::{IdleDim, Wallpaper};
use sill_ui_kit::harness::{SurfaceHarness, SurfaceSetup};
use sill_ui_kit::links::ControlCenterLink;
use sill_ui_kit::surface::Surface;

const VIEW: Viewport = Viewport {
    width: 480,
    height: 200,
    scale_percent: 100,
};

fn modality<S: Surface>(run: &SurfaceHarness<S>) -> Option<String> {
    run.harness.attr(".ds", "data-modality")
}

fn settle<S: Surface>(run: &mut SurfaceHarness<S>) {
    for _ in 0..3 {
        run.harness.advance(Duration::from_millis(50));
    }
}

/// A key turns the ring on, then a press on the root turns it off.
fn key_then_press<S: Surface>(setup: SurfaceSetup<S>) {
    let mut run = setup.viewport(VIEW).start();
    settle(&mut run);
    assert_eq!(modality(&run).as_deref(), Some("pointer"), "at first");
    run.harness.send(Input::key(ShortcutKey::Tab));
    settle(&mut run);
    assert_eq!(modality(&run).as_deref(), Some("keyboard"), "after a key");
    let at = run.harness.centre(".ds").expect("the root is laid out"); // test-only
    run.harness.send(Input::click(at));
    settle(&mut run);
    assert_eq!(modality(&run).as_deref(), Some("pointer"), "after a press");
}

/// The root's inline style carries the pixel tokens of a 1.25x output.
fn scaled<S: Surface>(setup: SurfaceSetup<S>) {
    let tokens = PixelToken::style_attr(Scale(150));
    assert!(!tokens.is_empty(), "1.25x writes tokens");
    let mut run = setup
        .viewport(Viewport {
            scale_percent: 125,
            ..VIEW
        })
        .start();
    settle(&mut run);
    let style = run.harness.attr(".ds", "style").unwrap_or_default();
    assert!(
        style.contains(&tokens),
        "the root sizes its tokens for 1.25x"
    );
}

#[test]
fn the_bar_follows_the_hosts_modality_and_scale() {
    key_then_press(SurfaceHarness::<Bar>::setup(ControlCenterLink::new(), ()));
    scaled(SurfaceHarness::<Bar>::setup(ControlCenterLink::new(), ()));
}

#[test]
fn the_dock_follows_the_hosts_modality_and_scale() {
    key_then_press(SurfaceHarness::<Dock>::setup((), ()));
    scaled(SurfaceHarness::<Dock>::setup((), ()));
}

#[test]
fn the_idle_dim_follows_the_hosts_modality_and_scale() {
    key_then_press(SurfaceHarness::<IdleDim>::setup((), ()));
    scaled(SurfaceHarness::<IdleDim>::setup((), ()));
}

#[test]
fn the_wallpaper_follows_the_hosts_modality_and_scale() {
    key_then_press(SurfaceHarness::<Wallpaper>::setup((), ()));
    scaled(SurfaceHarness::<Wallpaper>::setup((), ()));
}
