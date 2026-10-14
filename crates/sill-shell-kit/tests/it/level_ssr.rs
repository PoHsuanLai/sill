//! The capsule slider, the level indicator and the OSD card as markup (the user's brief of
//! 2026-09-25): each look and style, each glyph state, read-only and interactive, and the OSD
//! card shown at either anchor and hidden. Each golden is `tests/snapshots/level/<name>.html`, and every `ds-`
//! class in it must be styled by the stylesheet.
//!
//! `DS_BLESS=1 cargo test -p sill-shell-kit --test it level_ssr` rewrites the goldens.

use crate::support::specimens::{Checks, Specimen};

use dioxus::prelude::*;
use ds::assembly::ds::Inject;
use ds::base::vocab::Muting;
use ds::components::content::level_glyph::vocab::LevelGlyph;
use ds::components::controls::level_indicator::{LevelIndicator, LevelStyle};
use ds::components::controls::slider_model::SliderLook;
use ds::prelude::*;
use sill_shell_kit::osd::{OsdLevel, OsdPosition};
use sill_shell_kit::prelude::*;

#[derive(Props, Clone)]
struct HostProps {
    make: fn() -> Element,
}

impl PartialEq for HostProps {
    fn eq(&self, _: &Self) -> bool {
        false
    }
}

fn host(props: HostProps) -> Element {
    (props.make)()
}

fn render(make: fn() -> Element) -> String {
    let mut dom = VirtualDom::new_with_props(host, HostProps { make });
    dom.rebuild_in_place();
    dioxus_ssr::render(&dom)
}

const HEARD: LevelGlyph = LevelGlyph::Volume(Muting::Audible);

/// `body` under a transparent Osd root, as a shell surface draws it.
fn osd_root(body: Element) -> Element {
    rsx! {
        Ds { appearance: Appearance::default(), material: Material::Osd, chrome: Some(RootChrome::Transparent), stylesheet: Inject::Host,
            {body}
        }
    }
}

type Case = (&'static str, fn() -> Element);

const CASES: &[Case] = &[
    ("slider-capsule-volume-40", || {
        osd_root(
            rsx! { Slider { label: "Volume", value: Fraction(400), glyph: HEARD, look: SliderLook::Capsule } },
        )
    }),
    ("slider-capsule-knob-volume-40", || {
        osd_root(
            rsx! { Slider { label: "Volume", value: Fraction(400), glyph: HEARD, look: SliderLook::CapsuleKnob } },
        )
    }),
    ("indicator-discrete-volume-40", || {
        osd_root(
            rsx! { LevelIndicator { label: "Volume", value: Fraction(400), glyph: HEARD, style: LevelStyle::Discrete } },
        )
    }),
    ("slider-capsule-muted", || {
        osd_root(
            rsx! { Slider { label: "Volume", value: Fraction(400), glyph: LevelGlyph::Volume(Muting::Muted), look: SliderLook::Capsule } },
        )
    }),
    ("indicator-continuous-brightness-30", || {
        osd_root(
            rsx! { LevelIndicator { label: "Brightness", value: Fraction(300), glyph: LevelGlyph::Brightness } },
        )
    }),
    ("osd-top-right", || {
        osd_root(rsx! {
            Osd { shown: Shown::Visible, label: "MA270U", level: OsdLevel { value: Fraction(620), glyph: HEARD } }
        })
    }),
    ("osd-bottom-centre", || {
        osd_root(rsx! {
            Osd { shown: Shown::Visible, label: "Display", position: OsdPosition::BottomCentre, level: OsdLevel { value: Fraction(300), glyph: LevelGlyph::Brightness } }
        })
    }),
    ("osd-hidden", || {
        osd_root(rsx! {
            Osd { shown: Shown::Hidden, label: "Sound", level: OsdLevel { value: Fraction(500), glyph: HEARD } }
        })
    }),
    ("slider-mac-volume-40", || {
        osd_root(
            rsx! { Slider { label: "Volume", value: Fraction(400), glyph: HEARD, look: SliderLook::Mac } },
        )
    }),
];

pub(crate) fn specimens() -> Vec<Specimen> {
    CASES
        .iter()
        .map(|(name, make)| {
            Specimen::of(&format!("level/{name}.html"), render(*make)).checked(Checks::Styled)
        })
        .collect()
}

#[test]
fn an_indicator_takes_no_input_and_a_slider_does() {
    let indicator = render(CASES[4].1);
    assert!(indicator.contains("role=\"meter\""), "{indicator}");
    assert!(!indicator.contains("tabindex"), "{indicator}");
    assert!(!indicator.contains("ds-slider-hit"), "{indicator}");
    let slider = render(CASES[0].1);
    assert!(
        slider.contains("role=\"slider\"") && slider.contains("tabindex=\"0\""),
        "{slider}"
    );
    assert!(slider.contains("aria-valuenow=\"40\""), "{slider}");
}

#[test]
fn the_capsule_draws_its_glyph_twice_and_the_other_looks_once() {
    let mac = render(CASES.last().expect("the Mac look is last").1);
    assert_eq!(
        mac.matches("data-part=\"body\"").count(),
        1,
        "in the knob, once"
    );
    assert!(
        mac.contains("class=\"ds-slider-thumb\"><span class=\"ds-slider-icon\">"),
        "{mac}"
    );
    let parts = |html: &str| html.matches("data-part=\"body\"").count();
    assert_eq!(parts(&render(CASES[0].1)), 2, "on the well and in the fill");
    assert_eq!(parts(&render(CASES[1].1)), 1);
    assert_eq!(parts(&render(CASES[2].1)), 1);
    let segments = render(CASES[2].1);
    assert_eq!(
        segments
            .matches("class=\"ds-level-indicator-segment\"")
            .count(),
        16
    );
    assert_eq!(
        segments.matches("data-on=\"on\"").count() - 3,
        6,
        "6 squares and the 3 glyph parts shown at 40 %"
    );
}
