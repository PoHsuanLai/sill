//! The macOS polish pass (FINDINGS "Materials, blur and colour"), proved on a real Blitz document: a squircle
//! corner differs from a circle of the same radius at 45 degrees, a card stacks its hairline,
//! highlight and two shadows, a plate paints its gradient inside the superellipse, the launcher
//! card is as tall as its content, text menu rows are 22 px, a bar item draws its pill while
//! open, and a popover fades out before it closes.

use crate::support::probe;

use dioxus::prelude::*;
use ds::base::geometry::placement::Align;
use ds::base::geometry::placement::Flip;
use ds::base::geometry::placement::Side;
use ds::components::menus::item::item::MenuImage;
use ds::components::menus::palette::palette_group::PaletteGroup;
use ds::components::menus::palette::palette_group::PaletteRow;
use ds::host::measure::Anchor;
use ds::prelude::*;
use ds::root::common::Common;
use ds::style::appearance::blur::BlurState;
use ds::style::icon::family::PlateFamily;
use ds::style::tokens::shape::Corner;
use ds::style::tokens::shell_type::ShellType;
use ds::style::tokens::token::TokenScope;
use ds_harness::{Clock, Harness, HarnessConfig, Input, Viewport};
use image::{Rgba, RgbaImage};
use probe::{keep, rect};
use sill_shell_kit::prelude::*;
use std::time::Duration;

const VIEW: Viewport = Viewport {
    width: 640,
    height: 420,
    scale_percent: 100,
};

fn ms(n: u64) -> Duration {
    Duration::from_millis(n)
}

fn luma(pixel: Rgba<u8>) -> i32 {
    (i32::from(pixel[0]) * 3 + i32::from(pixel[1]) * 6 + i32::from(pixel[2])) / 10
}

/// The pixel at `(dx, dy)` from `selector`'s top-left corner.
fn at(frame: &RgbaImage, harness: &Harness, selector: &str, dx: f32, dy: f32) -> Rgba<u8> {
    let box_ = rect(harness, selector);
    *frame.get_pixel((box_.origin.x.0 + dx) as u32, (box_.origin.y.0 + dy) as u32)
}

/// No hairline and no shadow, through the material tokens a look sets: only the shape paints
/// outside the tint.
const BARE: &str = "--m-hairline-light:0; --m-hairline-dark:0; --m-shadow-strength:0;";

// ---- A squircle corner is not a circle -------------------------------------------------------

#[allow(non_snake_case)]
fn Corners() -> Element {
    rsx! {
        Ds { sheet: Some(sill_shell_kit::stylesheet()), appearance: Appearance::default(), material: Material::Window,
            div { style: "{BARE} display:flex; gap:40px; padding:40px; background:#ffffff",
                div { class: "squircle", style: "width:240px; height:200px",
                    Surface { material: Material::Dock, theme: Some(Scheme::Dark), radius: Some(Corner::Squircle(Px(40.0))),
                        div { style: "width:240px; height:200px" }
                    }
                }
                div { class: "circle", style: "width:240px; height:200px",
                    Surface { material: Material::Dock, theme: Some(Scheme::Dark), radius: Some(Corner::Px(Px(40.0))),
                        div { style: "width:240px; height:200px" }
                    }
                }
            }
        }
    }
}

/// Radius 40: the circle's 45 degree point is 11.7 px in from the corner on each axis, the
/// squircle's (extent 80, `n = 5`) 10.35 px. The pixel whose centre is at 11.5 lies inside the
/// squircle by 1.6 px along the diagonal and outside the circle, so the squircle paints it dark
/// where the circle leaves the white ground; well inside and well outside both agree.
#[test]
fn a_squircle_corner_is_fuller_than_a_circle_at_45_degrees() {
    let mut harness = Harness::new(Corners, HarnessConfig::new(VIEW).with_clock(Clock::Virtual));
    harness.advance(ms(60));
    let frame = harness.render().expect("a frame");
    keep(&frame, "squircle-corner");
    let (squircle, circle) = (".squircle > .ds", ".circle > .ds");
    let pair = |d: f32| {
        (
            luma(at(&frame, &harness, squircle, d, d)),
            luma(at(&frame, &harness, circle, d, d)),
        )
    };
    let (s, c) = pair(11.0);
    assert!(
        c - s > 60,
        "at the 45 degree point: squircle {s}, circle {c}"
    );
    let (s, c) = pair(3.0);
    assert!(s > 200 && c > 200, "outside both: {s} {c}");
    let (s, c) = pair(30.0);
    assert!(s < 80 && c < 80, "inside both: {s} {c}");
}

// ---- The material stack ----------------------------------------------------------------------

#[allow(non_snake_case)]
fn Card() -> Element {
    rsx! {
        Ds { sheet: Some(sill_shell_kit::stylesheet()), appearance: Appearance::default(), material: Material::Window,
            div { style: "padding:60px; background:#b4b4b4; height:400px",
                div { class: "card", style: "width:240px; height:120px",
                    Surface { material: Material::Popover, theme: Some(Scheme::Dark),
                        div { style: "width:240px; height:120px" }
                    }
                }
            }
        }
    }
}

/// A dark Popover card over a mid grey: its top row is the white highlight over the tint, the
/// pixel just outside its side is darker than the grey (the hairline), the pixel 2 px under it
/// is darker still (hairline and contact), and 30 px under it the ambient shadow still shows,
/// where 120 px away the grey is untouched.
#[test]
fn a_card_stacks_a_hairline_a_highlight_and_two_shadows() {
    let mut harness = Harness::new(Card, HarnessConfig::new(VIEW).with_clock(Clock::Virtual));
    harness.advance(ms(60));
    let frame = harness.render().expect("a frame");
    keep(&frame, "material-stack");
    let card = ".card > .ds";
    let box_ = rect(&harness, card);
    let (w, h) = (box_.size.width.0, box_.size.height.0);
    let top = luma(at(&frame, &harness, card, w / 2.0, 0.0));
    let middle = luma(at(&frame, &harness, card, w / 2.0, h / 2.0));
    assert!(top - middle > 8, "highlight {top} over the tint {middle}");
    let ground = luma(at(&frame, &harness, card, w / 2.0, h + 150.0));
    let side = luma(at(&frame, &harness, card, -1.0, h / 2.0));
    assert!(side < ground, "hairline {side} against the ground {ground}");
    let contact = luma(at(&frame, &harness, card, w / 2.0, h + 1.0));
    assert!(ground - contact > 10, "contact {contact} against {ground}");
    let ambient = luma(at(&frame, &harness, card, w / 2.0, h + 20.0));
    assert!(ground - ambient > 3, "ambient {ambient} against {ground}");
}

// ---- A plate --------------------------------------------------------------------------------

#[allow(non_snake_case)]
fn Plates() -> Element {
    rsx! {
        Ds { sheet: Some(sill_shell_kit::stylesheet()), appearance: Appearance::default(), material: Material::Window,
            div { style: "display:flex; gap:24px; padding:40px; background:#ffffff",
                span { class: "blue", IconView { source: IconSource::Glyph(Icon::Terminal), size: IconSize::Tile96, plate: Some(PlateFamily::Blue) } }
                span { class: "amber", IconView { source: IconSource::Glyph(Icon::Folder), size: IconSize::Tile48, plate: Some(PlateFamily::Amber) } }
            }
        }
    }
}

/// A 96 px blue plate: its corner pixel is the white ground (the superellipse leaves it), its
/// top-left inside is the light stop and its bottom-right the deep one, and its glyph is laid
/// out at 56 % of the plate.
#[test]
fn a_plate_is_a_gradient_superellipse_with_its_glyph_at_56_percent() {
    let mut harness = Harness::new(Plates, HarnessConfig::new(VIEW).with_clock(Clock::Virtual));
    harness.advance(ms(60));
    let frame = harness.render().expect("a frame");
    keep(&frame, "plates");
    let plate = ".blue .ds-plate";
    let corner = at(&frame, &harness, plate, 2.0, 2.0);
    assert!(
        luma(corner) > 200,
        "the corner shows the ground: {corner:?}"
    );
    let light = at(&frame, &harness, plate, 20.0, 20.0);
    let deep = at(&frame, &harness, plate, 76.0, 76.0);
    assert!(light[2] > 200 && light[0] < 120, "light stop: {light:?}");
    assert!(
        luma(deep) < luma(light),
        "deep {deep:?} under light {light:?}"
    );
    let glyph = rect(&harness, ".blue .ds-plate > .ds-ic");
    assert!((glyph.size.width.0 - 53.76).abs() < 1.0, "{glyph:?}");
    assert_eq!(rect(&harness, ".amber .ds-plate").size.width, Px(48.0));
}

// ---- Text menus and bar items ----------------------------------------------------------------

#[allow(non_snake_case)]
fn Bar() -> Element {
    rsx! {
        Ds { sheet: Some(sill_shell_kit::stylesheet()), appearance: Appearance::default(), material: Material::Bar,
            div { style: "display:flex; gap:8px; padding:4px 8px",
                MenuBarItem { common: Common { id: Some("open".to_string()), ..Common::default() }, shown: Shown::Visible, label: "Files", onclick: |_| {} }
                MenuBarItem { common: Common { id: Some("shut".to_string()), ..Common::default() }, label: "Edit", onclick: |_| {} }
            }
        }
    }
}

/// An open bar item paints the frame's pressed pill behind its text at the shell scale's 22 px
/// height (a Regular control, design/29-SIZING.md) and 13 px text; a closed one paints nothing.
#[test]
fn an_open_bar_item_draws_its_pill() {
    let mut harness = Harness::new(Bar, HarnessConfig::new(VIEW).with_clock(Clock::Virtual));
    harness.advance(ms(60));
    let frame = harness.render().expect("a frame");
    keep(&frame, "bar-item");
    assert_eq!(rect(&harness, "#open").size.height, Px(22.0));
    // The frame's grain speckles single pixels, so compare the padding's mean.
    let padding = |selector: &str| {
        let item = rect(&harness, selector);
        let seen = (4..20)
            .flat_map(|dy| (2..8).map(move |dx| (dx, dy)))
            .map(|(dx, dy)| {
                luma(*frame.get_pixel(
                    (item.origin.x.0 + dx as f32) as u32,
                    (item.origin.y.0 + dy as f32) as u32,
                ))
            })
            .collect::<Vec<_>>();
        seen.iter().sum::<i32>() / seen.len() as i32
    };
    let (open, shut) = (padding("#open"), padding("#shut"));
    assert!((open - shut).abs() > 4, "open {open} against closed {shut}");
}

#[allow(non_snake_case)]
fn SlimMenu() -> Element {
    let entries = vec![
        MenuItem::Item {
            value: 1,
            title: "New Window".to_owned(),
            image: None,
            key: None,
            check: None,
            availability: Availability::Enabled,
            hint: None,
            after: AfterPick::Close,
            text: Default::default(),
        },
        MenuItem::Separator,
        MenuItem::Item {
            value: 2,
            title: "Quit".to_owned(),
            image: Some(MenuImage::Icon(Icon::Power)),
            key: None,
            check: None,
            availability: Availability::Enabled,
            hint: None,
            after: AfterPick::Close,
            text: Default::default(),
        },
    ];
    rsx! {
        Ds { sheet: Some(sill_shell_kit::stylesheet()), appearance: Appearance::default(), material: Material::Window,
            div { id: "anchor", style: "width:20px; height:20px; margin:20px" }
            Menu::<i32> {
                placement: MenuPlacement::Popup,
                anchor: Anchor::Point(Point { x: Px(40.0), y: Px(40.0) }),
                items: entries,
                onpick: move |_| {},
                onclose: move |_| {},
            }
        }
    }
}

/// A Slim menu's rows are the shell type scale's menu row (28 px) and its separator row is the
/// hairline with 5 px above and below (11 px).
#[test]
fn text_menu_rows_are_the_shell_menu_row() {
    let mut harness = Harness::new(
        SlimMenu,
        HarnessConfig::new(VIEW).with_clock(Clock::Virtual),
    );
    harness.advance(ms(400));
    let rows = harness.count(".ds-menu-item");
    assert_eq!(rows, 2, "{}", harness.html());
    let first = rect(&harness, ".ds-menu-item");
    let row = Px(ShellType::MenuRow
        .fallback(TokenScope::BASE)
        .trim_end_matches("px")
        .parse()
        .expect("the menu row token is a plain length"));
    assert_eq!(first.size.height, row);
    let separator = rect(&harness, ".ds-menu-separator");
    assert_eq!(separator.size.height, Px(1.0));
    assert_eq!(separator.origin.y.0 - (first.origin.y.0 + row.0), 5.0);
}

// ---- The dock pill as a squircle root --------------------------------------------------------

#[allow(non_snake_case)]
fn DockPill() -> Element {
    rsx! {
        div { style: "padding:40px",
            div { class: "pill", style: "width:300px; height:60px",
                Ds { sheet: Some(sill_shell_kit::stylesheet()),
                    appearance: Appearance::default(),
                    material: Material::Dock,
                    blur: BlurState::Available,
                    radius: Some(Corner::Squircle(Px(18.0))),
                    div { style: "width:300px; height:60px" }
                }
            }
        }
    }
}

/// A dock root with a squircle corner (the frame masked, the shadows on the root): over a clear
/// backdrop its outermost corner pixel is empty, its middle is painted, and a pixel on the 45
/// degree line that a circle of radius 18 leaves empty (its centre 5.5 px in on each axis; the
/// circle's boundary is at 5.27, the squircle's at 4.66) carries paint.
#[test]
fn a_squircle_dock_root_masks_its_frame_and_keeps_its_shadow() {
    let mut harness = Harness::new(
        DockPill,
        HarnessConfig::new(VIEW).with_clock(Clock::Virtual),
    );
    harness.advance(ms(60));
    let frame = harness
        .render_over(ds_harness::Backdrop::Clear)
        .expect("a frame");
    keep(&frame, "dock-squircle");
    let pill = ".pill > .ds";
    assert!(
        at(&frame, &harness, pill, 0.0, 0.0)[3] < 90,
        "the corner is clear"
    );
    assert!(
        at(&frame, &harness, pill, 150.0, 30.0)[3] > 120,
        "the middle is painted"
    );
    assert!(
        at(&frame, &harness, pill, 5.0, 5.0)[3] > 60,
        "the 45 degree point is painted"
    );
    // The ambient shadow reaches under the pill, outside the mask.
    assert!(
        at(&frame, &harness, pill, 150.0, 68.0)[3] > 5,
        "the shadow is drawn"
    );
}

// ---- The launcher card sizes to its content --------------------------------------------------

#[allow(non_snake_case)]
fn Launcher() -> Element {
    let groups = vec![PaletteGroup::list(
        "Applications",
        vec![PaletteRow {
            detail: Some("File manager".into()),
            ..PaletteRow::new(1_u8, "Files")
        }],
    )];
    rsx! {
        Ds { sheet: Some(sill_shell_kit::stylesheet()), appearance: Appearance::default(), material: Material::Sheet, blur: BlurState::Available,
            div { class: "panel", style: "width:600px; height:400px",
                CommandPalette::<u8> {
                    label: "Launch".to_owned(),
                    placeholder: "Search".to_owned(),
                    query: "fi".to_owned(),
                    tokens: Vec::new(),
                    groups,
                    empty: "Nothing".to_owned(),
                    oninput: move |_| {},
                    onpick: move |_| {},
                    onclose: move |_| {},
                    host: CommandPaletteHost::Surface,
                    corner: Some(Corner::Squircle(Px(14.0))),
                }
            }
        }
    }
}

/// One result: the card is as tall as its field, header and row, far short of its 400 px
/// container, and as wide as it; the query is 22 px text beside a 20 px glyph.
#[test]
fn the_launcher_card_is_as_tall_as_its_content() {
    let mut harness = Harness::new(
        Launcher,
        HarnessConfig::new(VIEW).with_clock(Clock::Virtual),
    );
    harness.advance(ms(600));
    let card = rect(&harness, ".ds-palette");
    assert_eq!(card.size.width, Px(600.0));
    // About 160: field, header, one row and the list inset (12 since quire q20, concentric with
    // the card's 20 radius); the point is that it is nowhere near the 400 px container.
    assert!(card.size.height.0 < 172.0, "{card:?}");
    let glyph = rect(&harness, ".ds-text-field-icon .ds-ic");
    assert_eq!(glyph.size.width, Px(20.0));
    let field = rect(&harness, ".ds-text-field");
    assert!(field.size.height.0 >= 56.0, "{field:?}");
}

// ---- A popover fades before it goes ----------------------------------------------------------

#[allow(non_snake_case)]
fn Pop() -> Element {
    let mut open = use_signal(|| true);
    rsx! {
        Ds { sheet: Some(sill_shell_kit::stylesheet()), appearance: Appearance::default(), material: Material::Window,
            div { style: "padding:40px",
                span { id: "state", if open() { "open" } else { "closed" } }
            }
            if open() {
                Popover {
                    anchor: Anchor::Point(Point { x: Px(60.0), y: Px(60.0) }),
                    placement: Placement { side: Side::Bottom, align: Align::Start, flip: Flip::Allowed },
                    gap: Px(4.0),
                    onclose: move |()| open.set(false),
                    button { id: "inside", "Inside" }
                }
            }
        }
    }
}

/// A click outside a popover marks it leaving at once and closes it only after the `menu-out` fade.
#[test]
fn a_popover_fades_out_before_it_closes() {
    let mut harness = Harness::new(Pop, HarnessConfig::new(VIEW).with_clock(Clock::Virtual));
    harness.advance(ms(300));
    assert_eq!(harness.count("#inside"), 1);
    harness.send(Input::pointer_down(Point {
        x: Px(500.0),
        y: Px(20.0),
    }));
    harness.send(Input::pointer_up(Point {
        x: Px(500.0),
        y: Px(20.0),
    }));
    harness.advance(ms(16));
    assert_eq!(
        harness.attr(".ds-popover", "data-presence").as_deref(),
        Some("leaving")
    );
    assert_eq!(harness.text_of("#state").as_deref(), Some("open"));
    harness.advance(ms(400));
    assert_eq!(harness.text_of("#state").as_deref(), Some("closed"));
    assert_eq!(harness.count(".ds-popover"), 0);
}
