//! The launcher's stylesheet and its rendered panel (the preview pane up) under quire's
//! `Profile::Details` (design/26-DETAILS.md): `Strict` plus timing only by
//! the grammar's tokens (`Rule::OffGrammarTiming`) and no `infinite` loop (`Rule::InfiniteLoop`).

mod launcher_harness;

use std::time::Duration;

use ds::lint::{LintConfig, Profile, assert_clean, markup};
use sill_surfaces::links::LinkKey;

const LAUNCHER_CSS: &str = include_str!("../src/style/launcher.css");

fn details() -> LintConfig {
    LintConfig {
        profile: Profile::Details,
        ..LintConfig::new(&ds::kits())
    }
}

#[test]
fn the_launcher_stylesheet_is_clean_under_details() {
    assert_clean(LAUNCHER_CSS, &details());
}

#[test]
fn the_panel_with_its_pane_markup_is_clean_under_details() {
    let mut open = launcher_harness::open("m9");
    open.link.press(LinkKey::QuickLook);
    open.harness.advance(Duration::from_millis(60));
    let html = open.harness.html();
    assert!(html.contains("ds-preview"), "the pane is up");
    let css = format!("{}\n{}", ds::stylesheet(), LAUNCHER_CSS);
    let offences = markup(&html, &css, &details());
    assert!(offences.is_empty(), "{offences:#?}");
}
