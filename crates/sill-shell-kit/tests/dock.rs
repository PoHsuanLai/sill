//! The dock gaps on a real Blitz document: a material's corner set by
//! the caller, icons at the dock's sizes, and a dock label shown and hidden by the caller.

#[path = "support/probe.rs"]
mod probe;

use dioxus::prelude::*;
use ds::prelude::*;
use ds::style::icon::render::IconPx;
use ds::style::tokens::shape::Corner;
use ds_harness::{Clock, Driver, Harness, HarnessConfig, Input, Query, Viewport};
use image::{Rgba, RgbaImage};
use probe::rect;
use sill_shell_kit::prelude::*;
use std::time::Duration;

const VIEW: Viewport = Viewport {
    width: 480,
    height: 320,
    scale_percent: 100,
};

fn ms(n: u64) -> Duration {
    Duration::from_millis(n)
}

/// Two dark dock pills on a light window: the material's own 22 px corner, and a square one.
#[allow(non_snake_case)]
fn Pills() -> Element {
    rsx! {
        Ds { appearance: Appearance::default(), material: Material::Window,
            div { style: "display:flex; gap:40px; padding:20px",
                div { class: "own", style: "width:160px; height:72px",
                    Surface { material: Material::Dock, theme: Some(Scheme::Dark),
                        div { style: "width:160px; height:72px" }
                    }
                }
                div { class: "square", style: "width:160px; height:72px",
                    Surface {
                        material: Material::Dock,
                        theme: Some(Scheme::Dark),
                        radius: Some(Corner::Px(Px(0.0))),
                        div { style: "width:160px; height:72px" }
                    }
                }
            }
        }
    }
}

/// The pixel 2 px inside `selector`'s top-left corner.
fn corner(frame: &RgbaImage, harness: &Harness, selector: &str) -> Rgba<u8> {
    let at = rect(harness, selector);
    *frame.get_pixel(at.origin.x.0 as u32 + 2, at.origin.y.0 as u32 + 2)
}

/// Luma, enough to tell a dark tint from a light ground.
fn luma(pixel: Rgba<u8>) -> u32 {
    (u32::from(pixel[0]) * 3 + u32::from(pixel[1]) * 6 + u32::from(pixel[2])) / 10
}

/// `radius` reaches the material's paint: the square pill paints its corner, the pill with
/// the material's own 22 px leaves it to the ground.
#[test]
fn a_surfaces_radius_overrides_its_materials_corner() {
    let mut harness = Harness::new(Pills, HarnessConfig::new(VIEW).with_clock(Clock::Virtual));
    harness.advance(ms(40));
    let frame = harness.render().expect("a frame");
    let own = luma(corner(&frame, &harness, ".own > .ds"));
    let square = luma(corner(&frame, &harness, ".square > .ds"));
    assert!(
        own > 180,
        "the rounded corner shows the light ground: {own}"
    );
    assert!(square < 80, "the square corner is painted dark: {square}");
}

/// Icons at the dock's sizes: a glyph at `Tile96`, an image at a caller's own pixel size.
#[allow(non_snake_case)]
fn DockIcons() -> Element {
    rsx! {
        Ds { appearance: Appearance::default(), material: Material::Window,
            div { style: "display:flex; align-items:flex-start",
                span { class: "large", IconView { source: IconSource::Glyph(Icon::Star), size: IconSize::Tile96 } }
                span { class: "rest", IconView { source: IconSource::Glyph(Icon::Star), size: IconSize::Tile48 } }
                span { class: "exact", IconView { source: IconSource::Glyph(Icon::Star), size: IconSize::Px(IconPx(71)) } }
            }
        }
    }
}

/// A dock icon is drawn at the dock's size, not scaled up from 22.
#[test]
fn icons_take_the_docks_sizes() {
    let mut harness = Harness::new(
        DockIcons,
        HarnessConfig::new(VIEW).with_clock(Clock::Virtual),
    );
    harness.advance(ms(40));
    for (selector, side) in [
        (".large .ds-ic", 96.0),
        (".rest .ds-ic", 48.0),
        (".exact .ds-ic", 71.0),
    ] {
        let at = rect(&harness, selector);
        assert_eq!(
            (at.size.width, at.size.height),
            (Px(side), Px(side)),
            "{selector}"
        );
    }
}

/// Three tiles with a dock label each: one left to the pointer, one shown and one hidden by the
/// caller.
#[allow(non_snake_case)]
fn Labels() -> Element {
    rsx! {
        Ds { appearance: Appearance::default(), material: Material::Window,
            div { style: "display:flex; gap:120px; padding:80px 60px",
                div { class: "hovered",
                    DockLabel { text: "Files",
                        div { style: "width:48px; height:48px" }
                    }
                }
                div { class: "shown",
                    DockLabel { text: "Terminal", shown: Some(Shown::Visible),
                        div { style: "width:48px; height:48px" }
                    }
                }
                div { class: "hidden",
                    DockLabel { text: "Firefox", shown: Some(Shown::Hidden),
                        div { style: "width:48px; height:48px" }
                    }
                }
            }
        }
    }
}

/// `Shown::Visible` shows the label with no pointer on it; `Shown::Hidden` keeps it down under
/// the pointer, where an uncontrolled label shows after the Label profile's 100 ms.
#[test]
fn a_caller_shows_and_hides_a_dock_label() {
    let mut harness = Harness::new(Labels, HarnessConfig::new(VIEW).with_clock(Clock::Virtual));
    harness.advance(ms(400));
    assert_eq!(
        harness.count(".ds-dock-label"),
        1,
        "only the label its caller shows is up at rest"
    );
    let tile = harness.rect(".shown .ds-hover-target").expect("the tile");
    let label = harness
        .rect(".ds-dock-label")
        .expect("the label is laid out");
    assert!(
        label.origin.y.0 + label.size.height.0 <= tile.origin.y.0,
        "the label stands above its tile: {label:?} {tile:?}"
    );
    let hidden_target = harness
        .centre(".hidden .ds-hover-target")
        .expect("the hidden tile");
    harness.send(Input::pointer_move(hidden_target));
    harness.advance(ms(400));
    assert_eq!(
        harness.count(".ds-dock-label"),
        1,
        "a label its caller hides stays down under the pointer"
    );
    let target = harness
        .centre(".hovered .ds-hover-target")
        .expect("the tile");
    harness.send(Input::pointer_move(target));
    harness.advance(ms(60));
    assert_eq!(
        harness.count(".ds-dock-label"),
        1,
        "the Label profile waits 100 ms: not yet at 60"
    );
    harness.advance(ms(120));
    assert_eq!(
        harness.count(".ds-dock-label"),
        2,
        "an uncontrolled label shows under the pointer after 100 ms"
    );
}
