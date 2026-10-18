//! A switch of the root's scheme while the document is up repaints the glyphs and the text in the
//! new scheme's ink. Blitz bakes a text run's colour and an `<svg>`'s `currentColor` in when it
//! builds a box and a `color` change builds neither again (a transition under the change even bakes
//! the colour it was built at), so the host turns incremental layout off for the switch
//! (`ds_blitz::seam::follow_scheme`). Read off the painted pixels: a computed colour cannot see a
//! glyph painted in a stale one.

use dioxus::prelude::*;
use ds::components::controls::button_model::ImagePosition;
use ds::prelude::*;
use ds_harness::{Backdrop, Clock, Harness, HarnessConfig, Srgba, Viewport};
use sill_shell_kit::prelude::*;
use std::collections::HashMap;
use std::time::Duration;

static THEME: GlobalSignal<Theme> = Signal::global(|| Theme::Light);

const VIEW: Viewport = Viewport {
    width: 240,
    height: 60,
    scale_percent: 200,
};

/// WCAG's floor for a graphic.
const GRAPHIC: f32 = 3.0;

#[allow(non_snake_case)]
fn Bar() -> Element {
    rsx! {
        Ds { sheet: Some(sill_shell_kit::stylesheet()),
            appearance: Appearance { theme: THEME(), ..Appearance::default() },
            material: Material::Bar,
            div { style: "display:flex; gap:20px; padding:12px; align-items:center",
                MenuBarItem { label: "Desktop", onclick: |_| {} }
                MenuBarItem {
                    image: ImagePosition::Only,
                    icon: IconSource::Glyph(Icon::Search),
                    label: "Search",
                    onclick: |_| {},
                }
                WorkspacePills { label: "Workspaces",
                    WorkspacePill { label: "1", current: Selection::Selected, onclick: |_| {} }
                    WorkspacePill { label: "2", current: Selection::Unselected, onclick: |_| {} }
                }
            }
        }
    }
}

/// The best contrast any pixel of the first element matching `selector` has against the colour most
/// of its box shows.
fn peak_contrast(harness: &mut Harness, selector: &str) -> f32 {
    let rect = harness.rect(selector).expect("the element is drawn");
    let image = harness.render_over(Backdrop::Scheme).expect("renders");
    let scale = f32::from(VIEW.scale_percent) / 100.0;
    let span = |from: f32, size: f32, limit: u32| {
        ((from * scale) as u32)..((((from + size) * scale).ceil() as u32).min(limit))
    };
    let pixels: Vec<[u8; 3]> = span(rect.origin.y.0, rect.size.height.0, image.height())
        .flat_map(|y| span(rect.origin.x.0, rect.size.width.0, image.width()).map(move |x| (x, y)))
        .map(|(x, y)| {
            let [r, g, b, _] = image.get_pixel(x, y).0;
            [r, g, b]
        })
        .collect();
    let mut seen: HashMap<[u8; 3], usize> = HashMap::new();
    for pixel in &pixels {
        *seen.entry(*pixel).or_default() += 1;
    }
    let colour = |[r, g, b]: [u8; 3]| {
        Srgba([
            f32::from(r) / 255.0,
            f32::from(g) / 255.0,
            f32::from(b) / 255.0,
            1.0,
        ])
    };
    let ground = seen
        .into_iter()
        .max_by_key(|&(_, n)| n)
        .map(|(p, _)| p)
        .unwrap();
    pixels
        .iter()
        .map(|pixel| colour(*pixel).contrast(colour(ground)))
        .fold(1.0, f32::max)
}

const INKS: &[(&str, &str)] = &[
    (
        "the app name",
        ".ds-menu-bar-item[*|data-image=leading], .ds-menu-bar-item",
    ),
    ("the status glyph", "[*|aria-label=Search]"),
    ("the workspace pills", ".ds-ws-pills"),
];

fn weak(harness: &mut Harness) -> Vec<String> {
    INKS.iter()
        .map(|(name, selector)| (name, peak_contrast(harness, selector)))
        .filter(|&(_, got)| got < GRAPHIC)
        .map(|(name, got)| format!("{name}: {got:.2}"))
        .collect()
}

#[test]
fn a_root_switched_between_schemes_repaints_in_the_new_ink() {
    for (from, to) in [(Theme::Light, Theme::Dark), (Theme::Dark, Theme::Light)] {
        let mut harness = Harness::new(Bar, HarnessConfig::new(VIEW).with_clock(Clock::Virtual));
        harness.within(|| *THEME.write() = from);
        harness.advance(Duration::from_secs(1));
        assert_eq!(weak(&mut harness), Vec::<String>::new(), "{from:?}");
        harness.within(|| *THEME.write() = to);
        harness.advance(Duration::from_secs(1));
        assert_eq!(
            weak(&mut harness),
            Vec::<String>::new(),
            "{from:?} to {to:?}"
        );
    }
}
