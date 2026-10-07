//! The bar's glyphs and text in the dark scheme, read off the painted pixels: switching to Dark
//! from the control center (a live change, not a start in Dark) left the status icons and the
//! workspace pills in their light-scheme colours, dark on a dark bar.

use std::time::Duration;

use ds::prelude::*;
use ds_harness::Driver;
use ds_settings::Environment;
use sill_bar::Bar;
use sill_ui_kit::harness::SurfaceHarness;
use sill_ui_kit::links::ControlCenterLink;

/// WCAG's floor for a graphic (and large text): the status glyphs and the pills.
const GRAPHIC: f32 = 3.0;

/// What the bar draws that must read: the status items, the workspace pills, the clock.
const INKS: &[(&str, &str)] = &[
    ("the volume glyph", "[*|aria-label=Volume]"),
    ("the Wi-Fi glyph", "[*|aria-label=Network]"),
    ("the battery glyph", "[*|aria-label=Battery]"),
    ("the control center item", "[*|aria-label='Control Center']"),
    ("the workspace pills", ".ds-ws-pills"),
];

fn appearance(theme: Theme) -> Environment {
    let mut environment = Environment::default();
    environment.settings.appearance.theme = theme;
    environment
}

fn weak(run: &mut SurfaceHarness<Bar>) -> Vec<String> {
    INKS.iter()
        .filter_map(|(name, selector)| {
            let contrast = run.ink_contrast(selector);
            contrast
                .is_none_or(|got| got < GRAPHIC)
                .then(|| format!("{name} ({selector}): {contrast:?}"))
        })
        .collect()
}

fn bar_in(theme: Theme) -> SurfaceHarness<Bar> {
    let mut run = SurfaceHarness::<Bar>::setup(ControlCenterLink::new(), ())
        .appearance(sill_ui_kit::hooks::PinnedAppearance(appearance(theme)))
        .start();
    run.harness.advance(Duration::from_secs(1));
    run
}

#[test]
fn a_bar_that_starts_in_either_scheme_reads() {
    for theme in [Theme::Light, Theme::Dark] {
        let mut run = bar_in(theme);
        assert_eq!(weak(&mut run), Vec::<String>::new(), "{theme:?}");
    }
}

#[test]
fn a_bar_switched_to_the_other_scheme_reads() {
    for (from, to) in [(Theme::Light, Theme::Dark), (Theme::Dark, Theme::Light)] {
        let mut run = bar_in(from);
        run.set_appearance(appearance(to));
        run.harness.advance(Duration::from_secs(1));
        assert_eq!(weak(&mut run), Vec::<String>::new(), "{from:?} to {to:?}");
    }
}
