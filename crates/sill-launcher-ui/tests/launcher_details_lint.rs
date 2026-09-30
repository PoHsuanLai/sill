//! The launcher's stylesheet and its rendered panel (the preview pane up) under quire's
//! `Profile::Details` (design/26-DETAILS.md): `Strict` plus timing only by
//! the grammar's tokens (`Rule::OffGrammarTiming`) and no `infinite` loop (`Rule::InfiniteLoop`).

mod launcher_harness;

use ds_harness::Driver;
use std::time::Duration;

use ds_lint::{LintConfig, Profile, assert_clean, markup};

const LAUNCHER_CSS: &str = include_str!("../src/launcher/style.css");

fn details() -> LintConfig {
    LintConfig {
        profile: Profile::Details,
        ..LintConfig::new(&ds_shell::kits())
    }
}

#[test]
fn the_launcher_stylesheet_is_clean_under_details() {
    assert_clean(LAUNCHER_CSS, &details());
}

#[test]
fn the_panel_with_its_pane_markup_is_clean_under_details() {
    let mut open = launcher_harness::open("m9");
    open.press(launcher_harness::Press::QuickLook);
    open.harness.advance(Duration::from_millis(60));
    let html = open.harness.html();
    assert!(html.contains("ds-preview"), "the pane is up");
    let css = format!("{}\n{}", ds_shell::stylesheet(), LAUNCHER_CSS);
    let offences = markup(&html, &css, &details());
    assert!(offences.is_empty(), "{offences:#?}");
}
