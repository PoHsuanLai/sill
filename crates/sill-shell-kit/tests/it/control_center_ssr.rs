//! The control center's parts as markup: every control glyph renders and
//! lints clean; the goldens are `tests/snapshots/control_center/<name>.html`, each linted and
//! every `ds-` class in it styled by the stylesheet.
//!
//! `DS_BLESS=1 cargo test -p ds --features lint --test it control_center_ssr` rewrites the goldens.

use crate::support::specimens::Specimen;
#[path = "control_center/panels.rs"]
mod panels;
#[path = "control_center/panes.rs"]
mod panes;
#[path = "control_center/rows.rs"]
mod rows;
#[path = "control_center/tiles.rs"]
mod tiles;

use dioxus::prelude::*;
use ds::motion::pane_slide::Pane;
use ds::prelude::*;
use ds::style::icon::render::Glyph;
use ds::style::icon::render::GlyphProps;
use ds::style::icon::style::GlyphStyle;
use ds_lint::{LintConfig, Profile, markup};

/// `icon` at the bar's 22 px, rendered alone.
fn glyph(icon: Icon) -> String {
    let mut dom = VirtualDom::new_with_props(
        Glyph,
        GlyphProps {
            icon,
            size: IconSize::Bar,
            style: GlyphStyle::default(),
            cut: Default::default(),
        },
    );
    dom.rebuild_in_place();
    dioxus_ssr::render(&dom)
}

#[test]
fn every_control_glyph_renders_and_lints_clean() {
    let strict = LintConfig {
        profile: Profile::Strict,
        ..LintConfig::new(&sill_shell_kit::kits())
    };
    let mut failures = Vec::new();
    for icon in Icon::CONTROL.iter().copied() {
        let html = glyph(icon);
        let children = html.matches("<path").count()
            + html.matches("<rect").count()
            + html.matches("<circle").count();
        if children != icon.shapes_in(GlyphStyle::default()).len()
            || !html.contains("class=\"ds-ic\"")
        {
            failures.push(format!("{icon:?} drew {children} children: {html}"));
        }
        for offence in markup(&html, sill_shell_kit::stylesheet(), &strict) {
            failures.push(format!("{icon:?}: {:?} {}", offence.rule, offence.text));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// A rendered specimen: its golden name and markup.
type Rendered = (String, String);

fn tile_markup(case: tiles::TileCase) -> String {
    let mut dom = VirtualDom::new_with_props(tiles::tile, case);
    dom.rebuild_in_place();
    dioxus_ssr::render(&dom)
}

/// Every specimen, rendered.
fn rendered() -> Vec<Rendered> {
    let tiles = tiles::grid()
        .into_iter()
        .chain(tiles::chevrons())
        .map(|(name, case)| (name, tile_markup(case)));
    let rows = rows::CASES
        .into_iter()
        .map(|(case, name)| (name.to_owned(), row_markup(case)));
    let panes = panes::CASES
        .into_iter()
        .map(|(shown, name)| (name.to_owned(), pane_markup(shown)));
    let parts = panels::CASES
        .into_iter()
        .map(|(case, name)| (name.to_owned(), part_markup(case)));
    tiles.chain(rows).chain(panes).chain(parts).collect()
}

/// Every specimen with its golden.
pub(crate) fn specimens() -> Vec<Specimen> {
    rendered()
        .into_iter()
        .map(|(name, html)| Specimen::of(&format!("control_center/{name}.html"), html))
        .collect()
}

fn part_markup(case: panels::PartCase) -> String {
    let mut dom = VirtualDom::new_with_props(panels::part, panels::PartProps { case });
    dom.rebuild_in_place();
    dioxus_ssr::render(&dom)
}

fn pane_markup(shown: Pane) -> String {
    let mut dom = VirtualDom::new_with_props(panes::panes, panes::PaneProps { shown });
    dom.rebuild_in_place();
    dioxus_ssr::render(&dom)
}

fn row_markup(case: rows::RowCase) -> String {
    let mut dom = VirtualDom::new_with_props(rows::row, rows::RowProps { case });
    dom.rebuild_in_place();
    dioxus_ssr::render(&dom)
}

/// A tile says what it is in its attributes: the value as `data-state` and `aria-pressed`
/// (mixed while busy), its availability, the span, and the chevron's own name and
/// `aria-expanded`.
#[test]
fn a_tile_writes_its_state_span_and_chevron() {
    // (the specimen's word, data-state, data-availability, aria-pressed, has a ring)
    const CASES: &[(&str, &str, &str, &str, bool)] = &[
        ("off", "off", "enabled", "false", false),
        ("on", "on", "enabled", "true", false),
        ("busy", "off", "busy", "mixed", true),
    ];
    for (name, case) in tiles::grid() {
        let &(word, state, availability, pressed, ring) = CASES
            .iter()
            .find(|row| name.starts_with(&format!("tile-{}-", row.0)))
            .expect("a specimen of a known state");
        let html = tile_markup(case.clone());
        assert!(html.contains(&format!("data-state=\"{state}\"")), "{name}");
        assert!(
            html.contains(&format!("data-availability=\"{availability}\"")),
            "{name} {word}"
        );
        assert!(
            html.contains(&format!("aria-pressed=\"{pressed}\"")),
            "{name}"
        );
        assert_eq!(
            html.contains("ds-progress"),
            ring,
            "{name}: the ring turns only while busy"
        );
        assert!(html.contains("aria-label=\"Wi-Fi details\""), "{name}");
        assert!(html.contains("aria-expanded=\"false\""), "{name}");
    }
}

/// A row writes its trailing kind, a check row whether it is the chosen one, and a toggle row
/// names its switch by the row's title.
#[test]
fn a_row_writes_its_trailing_mark() {
    let cases = [
        (
            rows::RowCase::CheckOn,
            "data-trailing=\"check\"",
            Some("aria-checked=\"true\""),
        ),
        (
            rows::RowCase::CheckOff,
            "data-trailing=\"check\"",
            Some("aria-checked=\"false\""),
        ),
        (
            rows::RowCase::Toggle,
            "data-trailing=\"toggle\"",
            Some("aria-label=\"Headphones\""),
        ),
        (rows::RowCase::Chevron, "data-trailing=\"chevron\"", None),
        (rows::RowCase::Value, ">84%</span>", None),
        (rows::RowCase::Disabled, "aria-disabled=\"true\"", None),
    ];
    for (case, want, also) in cases {
        let html = row_markup(case);
        assert!(html.contains(want), "{case:?}: {html}");
        if let Some(also) = also {
            assert!(html.contains(also), "{case:?}: {html}");
        }
    }
    assert!(!row_markup(rows::RowCase::None).contains("data-trailing"));
}

/// A switcher mounted on a pane draws that pane alone, at rest, and moves nothing.
#[test]
fn a_switcher_at_rest_draws_one_pane() {
    for (shown, name) in panes::CASES {
        let html = pane_markup(shown);
        assert_eq!(html.matches("class=\"ds-pane\"").count(), 1, "{name}");
        assert!(
            html.contains(&format!("data-pane=\"{}\"", shown.slug())),
            "{name}"
        );
        assert!(html.contains("data-presence=\"present\""), "{name}");
        assert!(!html.contains("data-moving"), "{name}");
    }
}
