//! The control center's tile glyphs in the dark scheme, read off the painted pixels: after the
//! panel switched to Dark (its own theme control does this while it is open) the Focus moon and the
//! Wi-Fi glyph kept their light-scheme colours.

use crate::cc_support;

use std::time::Duration;

use cc_support::{fake, live};
use ds::prelude::*;
use ds_harness::Driver;
use sill_ui_kit::harness::ink_contrast;

/// WCAG's floor for a graphic.
const GRAPHIC: f32 = 3.0;

/// The glyphs the panel draws on its tiles and sliders.
const INKS: &[(&str, &str)] = &[
    (
        "the Wi-Fi tile glyph",
        ".ds-module-disc > .ds-status-glyph[*|data-kind=wifi]",
    ),
    ("the Focus moon", ".ds-module-disc > svg.ds-ic"),
    (
        "the Sound header glyph",
        ".ds-level-glyph[*|data-size='16']",
    ),
];

fn weak(run: &mut live::Running) -> Vec<String> {
    INKS.iter()
        .filter_map(|(name, selector)| {
            let contrast = ink_contrast(&mut run.harness, 100, selector);
            contrast
                .is_none_or(|got| got < GRAPHIC)
                .then(|| format!("{name} ({selector}): {contrast:?}"))
        })
        .collect()
}

fn panel() -> live::Running {
    let (services, feeds) = fake::services();
    let mut run = live::start(services, feeds);
    run.harness.advance(Duration::from_millis(500));
    run
}

#[test]
fn the_panel_reads_in_the_light_scheme() {
    assert_eq!(weak(&mut panel()), Vec::<String>::new());
}

#[test]
fn the_panel_switched_to_dark_reads() {
    let mut run = panel();
    run.set_theme(Theme::Dark);
    run.harness.advance(Duration::from_secs(1));
    assert_eq!(weak(&mut run), Vec::<String>::new());
}
