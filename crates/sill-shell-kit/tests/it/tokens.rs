//! The token blocks of the generated stylesheet agree with the Rust table and with each other:
//! a dark or level block never introduces a name the light block lacks, each accent block sets
//! exactly its four, each picker swatch is its accent's own colour, and every variable the
//! stylesheet reads is declared by it or written inline by the root.

use ds::prelude::*;
use ds::style::space::frame_vars::FrameVars;
use ds::style::tokens::accent_table::accent_of;
use ds::style::tokens::colour::ColourToken;
use ds::style::tokens::easing::EasingToken;
use ds::style::tokens::elevation::Shadow;
use ds::style::tokens::label_hue::HueMember;
use ds::style::tokens::label_hue::LabelHue;
use ds::style::tokens::layer::ZLayer;
use ds::style::tokens::person::PersonSwatch;
use ds::style::tokens::shape::Radius;
use ds::style::tokens::spacing::SpacingToken;
use ds::style::tokens::timing::DurationToken;
use ds::style::tokens::token::{Token, TokenScope};
use ds::style::tokens::type_scale::{Family, FontSize};
use ds::style::tokens::widget_paint::WidgetPaint;
use sill_shell_kit::stylesheet;
use std::collections::{BTreeMap, BTreeSet};

/// Every rule outside `@keyframes`, as its selector and its declarations in order.
fn rules(css: &str) -> Vec<(String, Vec<(String, String)>)> {
    let css = strip_comments(css);
    let mut out = Vec::new();
    let mut rest = css.as_str();
    while let Some(open) = rest.find('{') {
        let selector = rest[..open].trim().to_owned();
        if selector.starts_with("@layer") {
            // The design system's layer: its rules are the ones read, so step inside.
            rest = &rest[open + 1..];
            continue;
        }
        let Some(close) = matching(&rest[open..]) else {
            break;
        };
        let body = &rest[open + 1..open + close];
        if !selector.starts_with('@') {
            out.push((selector, declarations(body)));
        }
        rest = &rest[open + close + 1..];
    }
    out
}

fn declarations(body: &str) -> Vec<(String, String)> {
    body.split(';')
        .filter_map(|pair| pair.split_once(':'))
        .map(|(name, value)| (name.trim().to_owned(), value.trim().to_owned()))
        .collect()
}

fn matching(text: &str) -> Option<usize> {
    let mut depth = 0usize;
    for (index, byte) in text.bytes().enumerate() {
        match byte {
            b'{' => depth += 1,
            b'}' => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    return Some(index);
                }
            }
            _ => {}
        }
    }
    None
}

fn strip_comments(css: &str) -> String {
    let mut out = String::with_capacity(css.len());
    let mut rest = css;
    while let Some(start) = rest.find("/*") {
        out.push_str(&rest[..start]);
        match rest[start + 2..].find("*/") {
            Some(end) => rest = &rest[start + 2 + end + 2..],
            None => return out,
        }
    }
    out.push_str(rest);
    out
}

/// The declarations of every rule with exactly this selector, in cascade order (the dark
/// token block and the dark swatches share `.ds[*|data-theme=dark]`).
fn block(selector: &str) -> BTreeMap<String, String> {
    let found: Vec<_> = rules(stylesheet())
        .into_iter()
        .filter(|(sel, _)| sel == selector)
        .collect();
    assert!(!found.is_empty(), "no rule for {selector}");
    found.into_iter().flat_map(|(_, decls)| decls).collect()
}

/// Every custom property declared on bare `.ds`: the token block and the swatches (the reset's
/// `.ds` base text carries no custom property, so it adds nothing here).
fn token_block() -> BTreeMap<String, String> {
    rules(stylesheet())
        .into_iter()
        .filter(|(sel, decls)| sel == ".ds" && decls.iter().any(|(n, _)| n.starts_with("--")))
        .flat_map(|(_, decls)| decls)
        .collect()
}

#[test]
fn every_dark_and_level_name_has_a_light_value() {
    let light = token_block();
    for selector in [".ds[*|data-theme=dark]", ".ds[*|data-motion=reduced]"] {
        let overrides = block(selector);
        assert!(!overrides.is_empty(), "{selector} is empty");
        for name in overrides.keys().filter(|name| name.starts_with("--")) {
            assert!(
                light.contains_key(name),
                "{selector} sets {name}, .ds does not"
            );
        }
    }
}

#[test]
fn each_accent_block_sets_exactly_the_seven() {
    let seven: BTreeSet<&str> = [
        "--accent",
        "--accent-ink",
        "--accent-soft",
        "--accent-text",
        "--accent-text-material",
        "--accent-ring",
        "--seal",
    ]
    .into();
    let blocks: Vec<_> = rules(stylesheet())
        .into_iter()
        .filter(|(sel, _)| sel.contains("data-accent="))
        .collect();
    // One block per accent and scheme, and the same again under high contrast.
    assert_eq!(blocks.len(), 2 * Accent::ALL.len() * Scheme::ALL.len());
    for (selector, decls) in blocks {
        let names: BTreeSet<&str> = decls.iter().map(|(name, _)| name.as_str()).collect();
        assert_eq!(names, seven, "{selector}");
        assert_eq!(decls.len(), 7, "{selector} repeats a property");
    }
}

#[test]
fn each_accent_block_is_the_bands_roles() {
    for scheme in Scheme::ALL.iter().copied() {
        let theme = match scheme {
            Scheme::Light => "",
            Scheme::Dark => "[*|data-theme=dark]",
        };
        for accent in Accent::ALL.iter().copied() {
            let selector = format!(".ds{theme}[*|data-accent={}]", accent.slug());
            let decls = block(&selector);
            let want = accent_of(accent, scheme);
            for (name, value) in [
                ("--accent", want.fill.css()),
                ("--accent-ink", want.ink.css()),
                ("--accent-soft", want.wash_colour().css()),
                ("--accent-text", want.text.css()),
                ("--accent-text-material", want.text_material.css()),
                ("--accent-ring", want.ring_colour().css()),
                ("--seal", want.fill.css()),
            ] {
                assert_eq!(decls.get(name), Some(&value), "{selector} {name}");
            }
        }
    }
}

#[test]
fn each_swatch_is_its_accents_own_colour() {
    for scheme in Scheme::ALL.iter().copied() {
        let (swatches, theme) = match scheme {
            Scheme::Light => (
                rules(stylesheet())
                    .into_iter()
                    .filter(|(sel, decls)| {
                        sel == ".ds" && decls.iter().any(|(n, _)| n.starts_with("--swatch-"))
                    })
                    .flat_map(|(_, decls)| decls)
                    .collect::<BTreeMap<_, _>>(),
                "",
            ),
            Scheme::Dark => {
                let dark: BTreeMap<_, _> = rules(stylesheet())
                    .into_iter()
                    .filter(|(sel, decls)| {
                        sel == ".ds[*|data-theme=dark]"
                            && decls.iter().any(|(n, _)| n.starts_with("--swatch-"))
                    })
                    .flat_map(|(_, decls)| decls)
                    .collect();
                (dark, "[*|data-theme=dark]")
            }
        };
        for accent in Accent::ALL.iter().copied() {
            let swatch = swatches.get(&format!("--swatch-{}", accent.slug()));
            let accent_block = block(&format!(".ds{theme}[*|data-accent={}]", accent.slug()));
            assert_eq!(
                swatch,
                accent_block.get("--accent"),
                "{scheme:?} {accent:?}: the swatch is not the accent"
            );
            assert!(swatch.is_some(), "{scheme:?} {accent:?}: no swatch");
        }
    }
}

#[test]
fn the_token_block_holds_the_rust_table() {
    let light = token_block();
    let dark = block(".ds[*|data-theme=dark]");
    for token in ColourToken::ALL.iter().copied() {
        if token == ColourToken::AccentRing {
            continue; // Mixed from the resolved --accent; see `each_var_…` and `sill_shell_kit::kits()`.
        }
        for (scheme, over) in [(Scheme::Light, None), (Scheme::Dark, Some(&dark))] {
            let name = token.var().as_str();
            let got = over
                .and_then(|over| over.get(name))
                .or_else(|| light.get(name));
            assert_eq!(got, Some(&token.value(scheme).css()), "{name} {scheme:?}");
        }
    }
    for hue in LabelHue::ALL.iter().copied() {
        for member in [HueMember::Base, HueMember::Deep, HueMember::Soft] {
            let name = hue.colour(member).var().as_str();
            assert_eq!(
                light.get(name),
                Some(&hue.value(member, Scheme::Light).css()),
                "{name}"
            );
            assert_eq!(
                dark.get(name),
                Some(&hue.value(member, Scheme::Dark).css()),
                "{name} dark"
            );
        }
    }
}

/// The eight person swatches are declared once, on the light block, and the dark block does not
/// redeclare them: identity is data, not theme (design/03-COLOR.md section 13).
#[test]
fn the_person_swatches_are_declared_in_both_schemes_alike() {
    let light = token_block();
    let dark = block(".ds[*|data-theme=dark]");
    for swatch in PersonSwatch::ALL.iter().copied() {
        let name = swatch.var().as_str();
        assert_eq!(light.get(name), Some(&swatch.hex().css()), "{name}");
        assert_eq!(dark.get(name), None, "{name} is redeclared in dark");
    }
}

#[test]
fn every_table_name_is_declared_on_the_root() {
    let light = token_block();
    let names = ColourToken::ALL
        .iter()
        .map(|t| t.var())
        .chain(DurationToken::ALL.iter().map(|t| t.var()))
        .chain(EasingToken::ALL.iter().map(|t| t.var()))
        .chain(Radius::ALL.iter().map(|t| t.var()))
        .chain(SpacingToken::ALL.iter().map(|t| t.var()))
        .chain(Shadow::ALL.iter().map(|t| t.var()))
        .chain(WidgetPaint::ALL.iter().map(|t| t.var()))
        .chain(FontSize::ALL.iter().map(|t| t.var()))
        .chain(ZLayer::ALL.iter().map(|t| t.var()))
        .chain(Family::ALL.iter().map(|t| t.var()));
    for name in names {
        assert!(
            light.contains_key(name.as_str()),
            "{} is not declared",
            name.as_str()
        );
    }
}

/// Custom properties an element sets on itself inline, per instance, which no stylesheet block
/// could declare: the spark angle, the heal distance and index, the stagger indices
/// (design/05-MOTION.md section 5, rows 3, 10, 11 and 15), a `Fraction`'s `--f` and the
/// avatar's computed colours (design/04-COMPONENTS.md "Shared vocabulary" and section 11), and
/// an external icon's size (`IconView`, design/08-ICONS.md section 1.5), and a Space dot's
/// stops (`SpaceDot`, the editor's presets, handles, discs and swatch), and a
/// tinted plate's per-scheme stops and ink (`IconView { plate_tint }`), and a collection's
/// thumbnail size and pitch (`CollectionView`).
const PER_ELEMENT: &[&str] = &[
    // A chip's leading colour dot (quire `Chip { dot }`).
    "--chip-dot",
    "--a",
    "--dy",
    "--d",
    "--i",
    "--j",
    "--f",
    // The level control's rubber band, written only while it stretches.
    "--rb",
    "--av-bg",
    "--av-fg",
    "--tile-bg",
    "--tile-top",
    "--ic-size",
    "--dot-c1",
    "--dot-c2",
    "--dot-c3",
    "--plate-base-l",
    "--plate-deep-l",
    "--plate-ink-l",
    "--plate-base-d",
    "--plate-deep-d",
    "--plate-ink-d",
    // A notification group's layer count (`NotificationCard`).
    "--layers",
    "--swipe-dx",
    // A surface driven by the presence spring: 0 gone to 1 in place (`EdgePeek`).
    "--present-p",
    // An animated emoji's disc (`AnimatedEmoji { disc }`, design/25).
    "--em-disc",
    // A stepping spinner's angle (`ProgressIndicator`, design/26 R4).
    "--turn",
    // A running loop's step: the barber pole's slide (`ProgressIndicator`).
    "--step",
    // The voice orb's size, size-derived look values and turn (`VoiceOrb`, design/30 section 2.9).
    "--orb-size",
    "--orb-blur",
    "--orb-contrast",
    "--orb-dot",
    "--orb-shadow",
    "--orb-mask",
    "--orb-turn",
    // A loaded stretch's ends and the pointer's place under a drag (`Scrubber`).
    "--from",
    "--to",
    "--tip",
    // A collection's thumbnail size and grid pitch (`CollectionView`).
    "--cv-thumb",
    "--cv-pitch",
];

#[test]
fn every_var_the_stylesheet_reads_is_declared() {
    let css = strip_comments(stylesheet());
    let declared: BTreeSet<String> = rules(&css)
        .into_iter()
        .flat_map(|(_, decls)| decls)
        .map(|(name, _)| name)
        .filter(|name| name.starts_with("--"))
        .collect();
    // What the root writes inline: ask the program, not a copy of its list.
    let look = SpaceLook::default();
    let inline: BTreeSet<String> = FrameVars::of(&look, Scheme::Light)
        .style_attr()
        .split(';')
        .filter_map(|pair| pair.split_once(':'))
        .map(|(name, _)| name.to_owned())
        .collect();
    let mut missing = Vec::new();
    let mut rest = css.as_str();
    while let Some(start) = rest.find("var(") {
        let after = &rest[start + 4..];
        let end = after.find([')', ',']).unwrap_or(after.len());
        let name = after[..end].trim();
        let has_fallback = after[end..].starts_with(',');
        if !(declared.contains(name)
            || inline.contains(name)
            || PER_ELEMENT.contains(&name)
            || has_fallback)
        {
            missing.push(name.to_owned());
        }
        rest = after;
    }
    missing.sort();
    missing.dedup();
    assert!(missing.is_empty(), "read but never declared: {missing:?}");
}

/// The spacing scale is design/01-LAYOUT.md section 2's common steps, verbatim, plus the three
/// values 04-COMPONENTS quotes for a quire component (1.5, 13, 15), each emitted on `.ds` as
/// its own pixel value and nowhere else (it follows neither scheme nor motion).
#[test]
fn the_spacing_scale_is_the_layout_docs() {
    const STEPS: [&str; 27] = [
        "1", "1.5", "2", "3", "4", "5", "6", "7", "8", "9", "10", "11", "12", "13", "14", "15",
        "16", "18", "20", "22", "24", "26", "28", "32", "36", "40", "48",
    ];
    let written: Vec<String> = SpacingToken::ALL
        .iter()
        .map(|step| step.css_value(TokenScope::BASE).to_string())
        .collect();
    assert_eq!(written, STEPS.map(|step| format!("{step}px")));
    let light = token_block();
    for step in STEPS {
        let name = format!("--s-{}", step.replace('.', "-"));
        assert_eq!(light.get(&name), Some(&format!("{step}px")), "{name}");
    }
    for selector in [".ds[*|data-theme=dark]", ".ds[*|data-motion=reduced]"] {
        assert!(
            !block(selector).keys().any(|name| name.starts_with("--s-")),
            "{selector} overrides a spacing step"
        );
    }
}
