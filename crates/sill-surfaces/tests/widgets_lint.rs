//! Coherence rule 1 (quire CONSUMING.md §5) for the widgets (M10): every widget's stylesheet,
//! the frame's and the desktop layer's under `Profile::Strict`. The widgets' markup lint joins
//! this file as lanes a and b draw them.

use ds::lint::{LintConfig, Profile, assert_clean};
use sill_surfaces::surfaces::desktop_widgets::DESKTOP_WIDGETS_CSS;
use sill_surfaces::widgets::widgets_css;

fn strict() -> LintConfig {
    LintConfig {
        profile: Profile::Strict,
        ..LintConfig::default()
    }
}

#[test]
fn the_widget_stylesheets_are_clean_under_strict() {
    assert_clean(&widgets_css(), &strict());
    assert_clean(DESKTOP_WIDGETS_CSS, &strict());
}
