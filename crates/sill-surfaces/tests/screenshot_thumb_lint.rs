//! Coherence rules 1 and 2 for the screenshot thumbnail: its
//! stylesheet under `Profile::Strict`, and the real thumbnail through the markup lint, showing a
//! hovered card with its Delete action (the harness).

mod screenshot_thumb_support;

use std::time::Duration;

use ds::lint::{LintConfig, Profile, Rule, assert_clean, markup};
use screenshot_thumb_support::start;
use sill_settings::ShellFile;
use sill_surfaces::surfaces::screenshot_thumb::SCREENSHOT_THUMB_CSS;

#[test]
fn the_thumbnail_stylesheet_is_clean_under_strict() {
    let strict = LintConfig {
        profile: Profile::Strict,
        ..LintConfig::default()
    };
    assert_clean(SCREENSHOT_THUMB_CSS, &strict);
}

/// A shown card, the pointer on it.
fn shown_html() -> String {
    let mut running = start(ShellFile::default());
    running.one_up();
    let card = running.card();
    running.harness.pointer_move(card);
    running.harness.advance(Duration::from_millis(300));
    running.harness.html()
}

#[test]
fn the_shown_card_is_quire_markup_only() {
    let css = format!("{}\n{}", ds::stylesheet(), SCREENSHOT_THUMB_CSS);
    let html = shown_html();
    assert!(html.contains("ds-shot-image"), "{html}");
    let offences = markup(&html, &css, &LintConfig::default());
    let raw: Vec<_> = offences
        .iter()
        .filter(|o| o.rule == Rule::RawMarkup)
        .collect();
    assert!(raw.is_empty(), "{raw:#?}");
    assert!(offences.is_empty(), "{offences:#?}");
}
