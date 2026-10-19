//! The dock's glyph tiles in the dark scheme, read off the painted pixels: after a live switch to
//! Dark the Trash and the window glyph kept their light-scheme ink, dark on the dark dock.

use std::time::Duration;

use ds::prelude::*;
use ds_settings::Environment;
use sill_dock::Dock;
use sill_ui_kit::harness::{SurfaceHarness, fake};
use sill_ui_kit::hooks::PinnedAppearance;

/// WCAG's floor for a graphic.
const GRAPHIC: f32 = 3.0;

/// The glyph tiles: the Trash, and the window glyph an app without an icon gets.
const INKS: &[(&str, &str)] = &[
    (
        "the Trash glyph",
        ".ds-dock-tile[*|aria-label=Trash] svg.ds-ic",
    ),
    ("the first window glyph", ".ds-dock-tile svg.ds-ic"),
];

fn appearance(theme: Theme) -> Environment {
    let mut environment = Environment::default();
    environment.settings.appearance.theme = theme;
    environment
}

fn dock_in(theme: Theme) -> SurfaceHarness<Dock> {
    let mut run = SurfaceHarness::<Dock>::setup((), ())
        .appearance(PinnedAppearance(appearance(theme)))
        .start();
    run.feeds.apps.send_replace(fake::dock::index());
    run.feeds.pins.send_replace(fake::dock::pins());
    run.feeds.badges.send_replace(fake::dock::badges());
    run.feeds
        .windows
        .send_modify(|wl| wl.toplevels = fake::dock::toplevels_on("DP-1"));
    run.harness.advance(Duration::from_secs(1));
    run
}

fn weak(run: &mut SurfaceHarness<Dock>) -> Vec<String> {
    INKS.iter()
        .filter_map(|(name, selector)| {
            let contrast = run.ink_contrast(selector);
            contrast
                .is_none_or(|got| got < GRAPHIC)
                .then(|| format!("{name} ({selector}): {contrast:?}"))
        })
        .collect()
}

#[test]
fn a_dock_that_starts_in_either_scheme_reads() {
    for theme in [Theme::Light, Theme::Dark] {
        assert_eq!(weak(&mut dock_in(theme)), Vec::<String>::new(), "{theme:?}");
    }
}

#[test]
fn a_dock_switched_to_the_other_scheme_reads() {
    for (from, to) in [(Theme::Light, Theme::Dark), (Theme::Dark, Theme::Light)] {
        let mut run = dock_in(from);
        run.set_appearance(appearance(to));
        run.harness.advance(Duration::from_secs(1));
        assert_eq!(weak(&mut run), Vec::<String>::new(), "{from:?} to {to:?}");
    }
}
