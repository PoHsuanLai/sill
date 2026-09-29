//! Coherence rules 1 and 2 for the control center: its stylesheet under
//! `Profile::Strict`, and an SSR render of the real `ControlCenter` through the markup lint with
//! the popup document's sheets.

mod cc_support;

use cc_support::{fake, render};
use ds::lint::{LintConfig, Profile, Rule, assert_clean, markup};
use sill_settings::ShellFile;

const CONTROL_CENTER_CSS: &str = include_str!("../src/style/control_center.css");

fn strict() -> LintConfig {
    LintConfig {
        profile: Profile::Strict,
        ..LintConfig::new(&ds::kits())
    }
}

fn full_css() -> String {
    format!(
        "{}\n{}\n{}",
        ds::stylesheet(),
        sill_ui_kit::menu::POPUP_FRAME_CSS,
        CONTROL_CENTER_CSS
    )
}

fn html() -> String {
    let (services, _feeds) = fake::services();
    render(services, ShellFile::default())
}

#[test]
fn the_control_center_stylesheet_is_clean_under_strict() {
    assert_clean(CONTROL_CENTER_CSS, &strict());
}

#[test]
fn the_control_center_markup_lint_is_clean() {
    let offences = markup(&html(), &full_css(), &strict());
    assert!(offences.is_empty(), "{offences:#?}");
}

#[test]
fn the_control_center_renders_no_raw_markup() {
    let offences = markup(&html(), &full_css(), &strict());
    assert!(
        offences.iter().all(|o| o.rule != Rule::RawMarkup),
        "{offences:#?}"
    );
}
