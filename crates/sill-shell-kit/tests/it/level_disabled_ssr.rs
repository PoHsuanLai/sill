//! A disabled capsule `Slider` looked exactly like an enabled one at 0 %, so a brightness
//! level that could not move read as broken. Each state as markup, and the stylesheet rules that
//! draw the disabled one: the rail and lead glyph at .35, no knob, the not-allowed cursor, no
//! swell; a `ModulePanel` whose content is disabled dims its header glyph and percentage.

use dioxus::prelude::*;
use ds::assembly::ds::Inject;
use ds::base::vocab::Muting;
use ds::components::content::level_glyph::vocab::LevelGlyph;
use ds::components::controls::slider_model::SliderLook;
use ds::prelude::*;
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

/// `body` under a control center's root, as sill draws it.
fn rooted(body: Element) -> Element {
    rsx! {
        Ds { appearance: Appearance::default(), material: Material::Popover, stylesheet: Inject::Host,
            {body}
        }
    }
}

fn panel(availability: Availability) -> Element {
    rooted(rsx! {
        ModulePanel { glyph: Some(Icon::Sun), title: Some(TextLine::from("Display")), trailing: rsx! { "40%" }, availability,
            Slider {
                label: "Brightness",
                value: Fraction(0),
                glyph: LevelGlyph::Brightness,
                look: SliderLook::CapsuleKnob,
                availability,
            }
        }
    })
}

/// (case, markup, whether it is disabled)
type Case = (&'static str, fn() -> Element, Availability);

const CASES: &[Case] = &[
    (
        "enabled",
        || panel(Availability::Enabled),
        Availability::Enabled,
    ),
    (
        "disabled",
        || panel(Availability::Disabled),
        Availability::Disabled,
    ),
    (
        "disabled capsule",
        || {
            rooted(rsx! {
                Slider {
                    label: "Volume",
                    value: Fraction(0),
                    glyph: LevelGlyph::Volume(Muting::Audible),
                    availability: Availability::Disabled, look: SliderLook::Capsule }
            })
        },
        Availability::Disabled,
    ),
    ("busy", || panel(Availability::Busy), Availability::Busy),
];

#[test]
fn a_disabled_level_says_so_and_leaves_the_tab_order() {
    for (name, make, availability) in CASES {
        let html = render(*make);
        let level = html
            .split("class=\"ds-slider\"")
            .nth(1)
            .and_then(|rest| rest.split('>').next())
            .unwrap_or_else(|| panic!("{name}: no level in {html}"));
        match availability {
            Availability::Enabled => {
                assert!(!level.contains("aria-disabled"), "{name}: {level}");
                assert!(level.contains("tabindex=\"0\""), "{name}: {level}");
                assert!(!html.contains("aria-disabled"), "{name}: {html}");
            }
            Availability::Disabled => {
                assert!(level.contains("aria-disabled=\"true\""), "{name}: {level}");
                assert!(!level.contains("tabindex"), "{name}: {level}");
            }
            Availability::Busy => {
                assert!(level.contains("aria-busy=\"true\""), "{name}: {level}");
                assert!(!level.contains("aria-disabled"), "{name}: {level}");
                assert!(!level.contains("tabindex"), "{name}: {level}");
            }
        }
    }
    let panel = render(|| panel(Availability::Disabled));
    let head = panel
        .split("class=\"ds-module-panel\"")
        .nth(1)
        .and_then(|rest| rest.split('>').next())
        .expect("a panel");
    assert!(head.contains("aria-disabled=\"true\""), "{head}");
}

#[test]
fn the_stylesheet_draws_the_disabled_level_apart() {
    let css = sill_shell_kit::stylesheet();
    const RULES: &[&str] = &[
        ".ds-slider[*|aria-disabled=true]{ cursor:not-allowed; }",
        ".ds-slider[*|aria-disabled=true] .ds-slider-icon-lead{ opacity:.35; }",
        ".ds-slider[*|aria-disabled=true] .ds-slider-thumb{ display:none; }",
        ".ds-module-panel[*|aria-disabled=true] .ds-module-panel-trailing{ opacity:.35; }",
    ];
    for rule in RULES {
        assert!(css.contains(rule), "missing: {rule}");
    }
}
