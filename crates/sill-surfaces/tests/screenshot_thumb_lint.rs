//! Coherence rule 1 (quire CONSUMING.md §5) for the screenshot thumbnail (M10): its stylesheet
//! under `Profile::Strict`. Its markup lint joins this file as lane c draws it.

use ds::lint::{LintConfig, Profile, assert_clean};
use sill_surfaces::surfaces::screenshot_thumb::SCREENSHOT_THUMB_CSS;

#[test]
fn the_thumbnail_stylesheet_is_clean_under_strict() {
    let strict = LintConfig {
        profile: Profile::Strict,
        ..LintConfig::default()
    };
    assert_clean(SCREENSHOT_THUMB_CSS, &strict);
}
