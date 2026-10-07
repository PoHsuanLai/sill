//! The bar gaps' paint on a real Blitz document (FINDINGS "Bar gaps"): a bar
//! root tints itself with the Space gradient at its material alpha and cross-fades it on a look
//! change; a popover root paints nothing outside its card; the frame ground turns the paper inks
//! into the frame's, and overlays opened from it are paper again; a status item takes its box
//! and glyph from the consumer's two properties.

use crate::support::probe;

use dioxus::prelude::*;
use ds::base::geometry::placement::Align;
use ds::base::geometry::placement::Side;
use ds::components::controls::button_model::ImagePosition;
use ds::host::measure::Anchor;
use ds::prelude::*;
use ds::style::appearance::blur::BlurState;
use ds::style::space::frame_vars::FrameVars;
use ds::style::space::palette::derive;
use ds::style::space::presets::PRESETS;
use ds::style::tokens::colour::ColourToken;
use ds::style::tokens::easing::EasingToken;
use ds::style::tokens::hex::Hex;
use ds::style::tokens::status::StatusMetrics;
use ds::style::tokens::timing::DurationToken;
use ds_harness::{Backdrop, Clock, Driver, Harness, HarnessConfig, Input, Viewport};
use image::RgbaImage;
use probe::{distance, keep, modal, pixels, rect};
use sill_shell_kit::prelude::*;
use std::time::Duration;

const BAR_VIEW: Viewport = Viewport {
    width: 400,
    height: 40,
    scale_percent: 100,
};

const PANEL_VIEW: Viewport = Viewport {
    width: 320,
    height: 240,
    scale_percent: 100,
};

/// Test-only layout: the bar's height (sill writes it inline from `bar.height_px`), a spacer
/// that gives a popup root the room sill's popup gives it, and an ink swatch.
const PROBE_CSS: &str = ".bar-fill{display:flex;align-items:center;gap:8px;height:40px;padding-left:200px}\
    .room{height:200px}\
    .ink-probe{display:block;width:20px;height:20px;background:var(--ink)}";

/// A document under test.
type App = fn() -> Element;

fn ms(n: u64) -> Duration {
    Duration::from_millis(n)
}

fn light() -> Appearance {
    Appearance {
        theme: Theme::Light,
        ..Appearance::default()
    }
}

/// Preset `index`'s dots with no grain, so a pixel is the gradient alone.
fn look(index: usize) -> SpaceLook {
    SpaceLook {
        dots: PRESETS[index].dots.to_vec(),
        grain: Grain(0),
        ..SpaceLook::default()
    }
}

/// `colour` at `alpha` (thousandths) over white: what a tint over a white backdrop shows.
fn over_white(colour: Hex, alpha: u16) -> [u8; 4] {
    let a = f64::from(alpha) / 1000.0;
    let [r, g, b] = colour
        .0
        .map(|c| (a * f64::from(c) + (1.0 - a) * 255.0).round() as u8);
    [r, g, b, 255]
}

fn first_stop(index: usize) -> Hex {
    let stops = derive(&look(index).dots, Scheme::Light).stops;
    Hex::parse(&stops[0]).expect("a stop is a hex colour")
}

fn pixel(frame: &RgbaImage, x: u32, y: u32) -> [u8; 4] {
    frame.get_pixel(x, y).0
}

fn bar(blur: BlurState, index: usize) -> Element {
    rsx! {
        Ds { appearance: light(), look: look(index), material: Material::Bar, blur,
            style { {PROBE_CSS} }
            div { class: "bar-fill" }
        }
    }
}

#[allow(non_snake_case)]
fn BarOverBlur() -> Element {
    bar(BlurState::Available, 0)
}

#[allow(non_snake_case)]
fn BarSolid() -> Element {
    bar(BlurState::Unavailable, 0)
}

// ---- The tint at rest -----------------------------------------------------------------------

/// Over blur the bar is the gradient's first stop at the Bar material's .70 (light, at the
/// default key); without blur, at the solid floor .94 (design/21-SPACES.md section 3).
#[test]
fn the_bar_at_rest_is_the_first_stop_at_the_material_alpha() {
    // (app, name, alpha in thousandths)
    let cases: [(App, &str, u16); 2] = [(BarOverBlur, "over blur", 700), (BarSolid, "solid", 940)];
    let stop = first_stop(0);
    for (app, name, alpha) in cases {
        let mut harness =
            Harness::new(app, HarnessConfig::new(BAR_VIEW).with_clock(Clock::Virtual));
        harness.advance(ms(50));
        let frame = harness.render().expect("renders");
        keep(&frame, &format!("bar-tint-{}", name.replace(' ', "-")));
        let got = pixel(&frame, 1, 1);
        let want = over_white(stop, alpha);
        assert!(
            distance(got, want) <= 3,
            "{name}: the corner is {got:?}, the first stop {stop:?} at .{alpha} over white is {want:?}"
        );
        // And it is not the old flat surface tint the bar painted before.
        let surface = over_white(Hex([248, 249, 246]), alpha);
        assert!(distance(got, surface) > 6, "{name}: still the surface tint");
    }
}

// ---- The cross-fade -------------------------------------------------------------------------

#[allow(non_snake_case)]
fn SwitchingBar() -> Element {
    let mut index = use_signal(|| 0usize);
    rsx! {
        Ds { sheet: Some(sill_shell_kit::stylesheet()), appearance: light(), look: look(index()), material: Material::Bar, blur: BlurState::Available,
            style { {PROBE_CSS} }
            div { class: "bar-fill",
                MenuBarItem {
                    image: ImagePosition::Only,
                    icon: Icon::Grid,
                    label: "Next Space",
                    onclick: move |_| index.set(3),
                }
            }
        }
    }
}

/// When the cross-fade (`--t-big` with `--e-out`, design/21 section 5) shows the two Spaces
/// about equally: the first moment the curve reaches a quarter, asked of the token table. The
/// incoming layer is then at .25 over the outgoing one at .75, over the new gradient beneath
/// both, so the old Space weighs (1 - .25)^2 = .56 and the new one .44.
fn half_of_the_fade() -> Duration {
    let length = DurationToken::Big.duration(MotionLevel::Standard);
    let curve = EasingToken::Out.easing(MotionLevel::Standard);
    let whole = length.as_millis() as u64;
    let half = (1..=whole)
        .find(|&at| curve.at(Fraction((at * 1000 / whole) as u16)).0 >= 250)
        .unwrap_or(whole / 2);
    ms(half)
}

/// A look change cross-fades the tint over `--t-big` (400 ms at the standard level): partway,
/// the corner is between the two Spaces' colours and is neither (design/21 section 5).
#[test]
fn a_look_change_cross_fades_the_tint() {
    let mut harness = Harness::new(
        SwitchingBar,
        HarnessConfig::new(BAR_VIEW).with_clock(Clock::Virtual),
    );
    harness.advance(ms(50));
    let before = pixel(&harness.render().expect("renders"), 1, 1);
    let button = harness
        .centre(".ds-menu-bar-item")
        .expect("the switch is drawn");
    harness.send(Input::click(button));
    harness.advance(half_of_the_fade());
    let halfway = harness.render().expect("renders");
    keep(&halfway, "bar-cross-fade-halfway");
    let middle = pixel(&halfway, 1, 1);
    harness.advance(ms(700));
    let after = pixel(&harness.render().expect("renders"), 1, 1);
    assert!(
        distance(before, after) >= 8,
        "the two Spaces differ: {before:?} {after:?}"
    );
    let settled = over_white(first_stop(3), 700);
    assert!(
        distance(after, settled) <= 4,
        "at rest the new Space's first stop at .70 is {settled:?}, the corner {after:?}"
    );
    for channel in 0..3 {
        let (low, high) = (
            before[channel].min(after[channel]),
            before[channel].max(after[channel]),
        );
        assert!(
            (low.saturating_sub(3)..=high.saturating_add(3)).contains(&middle[channel]),
            "channel {channel}: halfway {middle:?} is not between {before:?} and {after:?}"
        );
    }
    assert!(
        distance(middle, before) > 2 && distance(middle, after) > 2,
        "halfway {middle:?} should be neither {before:?} nor {after:?}"
    );
}

// ---- A popup root ---------------------------------------------------------------------------

fn entries() -> Vec<MenuItem<u8>> {
    ["Wi-Fi settings", "Turn Wi-Fi off"]
        .into_iter()
        .enumerate()
        .map(|(value, title)| MenuItem::Item {
            value: value as u8,
            title: title.to_owned(),
            image: None,
            key: None,
            check: None,
            availability: Availability::Enabled,
            hint: None,
            after: AfterPick::Close,
            text: Default::default(),
        })
        .collect()
}

fn popup(chrome: Option<RootChrome>) -> Element {
    rsx! {
        Ds { appearance: light(), material: Material::Popover, chrome,
            style { {PROBE_CSS} }
            div { class: "room" }
            Menu::<u8> {
                placement: MenuPlacement::Popup,
                anchor: Anchor::Point(Point { x: Px(80.0), y: Px(60.0) }),
                items: entries(),
                onpick: |_| {},
                onclose: |_| {},
            }
        }
    }
}

#[allow(non_snake_case)]
fn PopupRoot() -> Element {
    popup(None)
}

#[allow(non_snake_case)]
fn PaintedPopupRoot() -> Element {
    popup(Some(RootChrome::Painted))
}

/// A Popover root paints nothing on its own box: over a clear backdrop, its corner outside the
/// card has alpha 0, while the card paints. The same root told to paint does paint
/// there, so the probe is not vacuous.
#[test]
fn a_popover_roots_corner_outside_its_card_is_transparent() {
    let mut harness = Harness::new(
        PopupRoot,
        HarnessConfig::new(PANEL_VIEW).with_clock(Clock::Virtual),
    );
    harness.advance(ms(80));
    let frame = harness.render_over(Backdrop::Clear).expect("renders");
    keep(&frame, "popup-root-clear");
    // Inside the root's rounded corner, where a painted root would paint.
    assert_eq!(pixel(&frame, 10, 20)[3], 0, "the root's corner");
    assert_eq!(pixel(&frame, 4, 236)[3], 0, "far from the card");
    let card = rect(&harness, ".ds-menu");
    let inside = modal(&pixels(&frame, card, 8.0));
    assert!(inside[3] > 200, "the card paints itself: {inside:?}");

    let mut painted = Harness::new(
        PaintedPopupRoot,
        HarnessConfig::new(PANEL_VIEW).with_clock(Clock::Virtual),
    );
    painted.advance(ms(80));
    let frame = painted.render_over(Backdrop::Clear).expect("renders");
    assert!(
        pixel(&frame, 10, 20)[3] > 200,
        "a painted root covers its box: {:?}",
        pixel(&frame, 10, 20)
    );
}

// ---- The frame ground -----------------------------------------------------------------------

#[allow(non_snake_case)]
fn GroundedBar() -> Element {
    rsx! {
        Ds { sheet: Some(sill_shell_kit::stylesheet()), appearance: light(), look: look(0), material: Material::Bar,
            style { {PROBE_CSS} }
            div { class: "bar-fill", span { class: "ink-probe on-frame" } }
            Popover {
                anchor: Anchor::Point(Point { x: Px(20.0), y: Px(60.0) }),
                placement: Placement::new(Side::Bottom, Align::Start),
                gap: Px(0.0),
                onclose: |_| {},
                div { style: "padding:12px", span { class: "ink-probe on-paper" } }
            }
        }
    }
}

/// On the bar `--ink` is the Space's `--f-ink`; in a popover opened from it, the paper ink again.
#[test]
fn the_frame_ground_draws_frame_inks_and_its_overlays_paper() {
    let mut harness = Harness::new(
        GroundedBar,
        HarnessConfig::new(PANEL_VIEW).with_clock(Clock::Virtual),
    );
    harness.advance(ms(400));
    let frame = harness.render().expect("renders");
    keep(&frame, "frame-ground");
    let frame_ink = Hex::parse(&FrameVars::of(&look(0), Scheme::Light).ink).expect("hex");
    let paper_ink = ColourToken::Ink.value(Scheme::Light);
    let on_frame = modal(&pixels(&frame, rect(&harness, ".on-frame"), 2.0));
    let on_paper = modal(&pixels(&frame, rect(&harness, ".on-paper"), 2.0));
    let [r, g, b] = frame_ink.0;
    assert!(
        distance(on_frame, [r, g, b, 255]) <= 2,
        "on the bar {on_frame:?}, --f-ink {frame_ink:?}"
    );
    let paper = Hex::parse(&paper_ink.css()).expect("the paper ink is a hex colour");
    let [r, g, b] = paper.0;
    assert!(
        distance(on_paper, [r, g, b, 255]) <= 2,
        "in the popover {on_paper:?}, --ink {paper:?}"
    );
    assert!(distance(on_frame, on_paper) > 6, "the two inks differ");
}

// ---- Status items ---------------------------------------------------------------------------

#[allow(non_snake_case)]
fn StatusItems() -> Element {
    let large = StatusMetrics {
        box_size: Px(30.0),
        glyph: Px(24.0),
    };
    rsx! {
        Ds { sheet: Some(sill_shell_kit::stylesheet()), appearance: light(), look: look(0), material: Material::Bar,
            style { {PROBE_CSS} }
            div { class: "bar-fill",
                div { class: "large", style: large.style_attr(),
                    MenuBarItem { image: ImagePosition::Only, icon: Icon::Wifi, label: "Network", onclick: |_| {} }
                }
                div { class: "plain",
                    MenuBarItem { image: ImagePosition::Only, icon: Icon::Wifi, label: "Network", onclick: |_| {} }
                }
            }
        }
    }
}

/// How wide the ink inside `box_rect` is: the glyph's drawn extent, not its element's.
fn inked_width(frame: &RgbaImage, box_rect: Rect) -> u32 {
    let ground = modal(&pixels(frame, box_rect, 0.0));
    let x0 = box_rect.origin.x.0 as u32;
    let y0 = box_rect.origin.y.0 as u32;
    let (w, h) = (box_rect.size.width.0 as u32, box_rect.size.height.0 as u32);
    let columns: Vec<u32> = (x0..x0 + w)
        .filter(|&x| (y0..y0 + h).any(|y| distance(pixel(frame, x, y), ground) > 40))
        .collect();
    match (columns.first(), columns.last()) {
        (Some(first), Some(last)) => last - first + 1,
        _ => 0,
    }
}

/// A status item is a slot `--bar-status-box` tall and at least the bar's 30 wide
/// (`--bar-status-w`, design/29-SIZING.md) holding a glyph drawn at `--bar-status-glyph`; without
/// the properties, the keys' defaults 22 and 16.
#[test]
fn a_status_item_takes_its_box_and_glyph_from_the_properties() {
    let mut harness = Harness::new(
        StatusItems,
        HarnessConfig::new(BAR_VIEW).with_clock(Clock::Virtual),
    );
    harness.advance(ms(50));
    let frame = harness.render().expect("renders");
    keep(&frame, "status-items");
    // (selector, box, glyph)
    const CASES: &[(&str, f32, f32)] = &[(".large", 30.0, 24.0), (".plain", 22.0, 16.0)];
    for &(scope, side, glyph) in CASES {
        let button = rect(&harness, &format!("{scope} .ds-menu-bar-item"));
        assert_eq!(
            (button.size.width, button.size.height),
            (Px(side.max(30.0)), Px(side)),
            "{scope}"
        );
        let svg = rect(&harness, &format!("{scope} .ds-ic"));
        assert_eq!(svg.size.width, Px(glyph), "{scope}");
        // The drawing scales with the element: the solid wifi spans 22 of its 24 units.
        let inked = inked_width(&frame, button);
        let want = (glyph * 22.0 / 24.0).round() as u32;
        assert!(
            inked.abs_diff(want) <= 2,
            "{scope}: the glyph is drawn {inked} wide, want about {want}"
        );
    }
}

/// Under the pointer a status item fills with `--f-pill-hover`.
#[test]
fn a_status_item_fills_under_the_pointer() {
    let mut harness = Harness::new(
        StatusItems,
        HarnessConfig::new(BAR_VIEW).with_clock(Clock::Virtual),
    );
    harness.advance(ms(50));
    let button = rect(&harness, ".plain .ds-menu-bar-item");
    let corner = |frame: &RgbaImage| {
        pixel(
            frame,
            button.origin.x.0 as u32 + 3,
            button.origin.y.0 as u32 + 3,
        )
    };
    let rest = corner(&harness.render().expect("renders"));
    harness.send(Input::pointer_move(
        harness.centre(".plain .ds-menu-bar-item").expect("drawn"),
    ));
    harness.advance(ms(400));
    let hovered = corner(&harness.render().expect("renders"));
    assert!(
        distance(rest, hovered) > 4,
        "rest {rest:?}, hovered {hovered:?}"
    );
}
