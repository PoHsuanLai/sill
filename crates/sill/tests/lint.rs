//! Coherence rules 1 and 2 for every surface, one loop over one table: its own stylesheet is
//! clean under `Profile::Strict`, and the markup it draws in each state worth checking is clean
//! against quire's classes and its own stylesheet. A surface's row names it; a failure names the
//! row and the state.

use std::time::Duration;

use ds::lint::{LintConfig, Profile, markup, stylesheet};
use sill_ui_kit::Osd;
use sill_ui_kit::harness::{SurfaceHarness, fake};
use sill_ui_kit::links::{LevelSource, OsdKind, OsdLink};

/// One surface's row: its stylesheet and a way to draw each state to lint.
struct SurfaceCase {
    name: &'static str,
    /// The surface's own stylesheet; empty when quire's alone styles it.
    css: &'static str,
    /// The document the surface draws in each state, named: what the harness's `html` says.
    states: fn() -> Vec<(&'static str, String)>,
}

const SURFACES: &[SurfaceCase] = &[SurfaceCase {
    name: "osd",
    css: sill_ui_kit::surfaces::osd::OSD_CSS,
    states: osd_states,
}];

fn strict() -> LintConfig {
    LintConfig {
        profile: Profile::Strict,
        ..LintConfig::new(&ds::kits())
    }
}

/// The indicator hidden, then shown for a volume change.
fn osd_states() -> Vec<(&'static str, String)> {
    let mut osd = SurfaceHarness::<Osd>::setup(OsdLink::new(), ()).start();
    osd.advance(Duration::from_millis(40));
    let hidden = osd.html();
    assert!(hidden.contains("data-shown=\"hidden\""), "{hidden}");
    osd.link
        .show(OsdKind::Volume, vec![fake::output().id], LevelSource::Live);
    osd.advance(Duration::from_millis(60));
    let shown = osd.html();
    assert!(shown.contains("data-presence=\"entering\""), "{shown}");
    vec![("hidden", hidden), ("shown", shown)]
}

#[test]
fn every_surface_stylesheet_is_clean_under_strict() {
    for case in SURFACES {
        let offences = stylesheet(case.css, &strict());
        assert!(offences.is_empty(), "{}: {offences:#?}", case.name);
    }
}

#[test]
fn every_surface_draws_clean_markup_in_each_state() {
    for case in SURFACES {
        let css = format!("{}\n{}", ds::stylesheet(), case.css);
        for (state, html) in (case.states)() {
            let offences = markup(&html, &css, &strict());
            assert!(
                offences.is_empty(),
                "{} ({state}): {offences:#?}",
                case.name
            );
        }
    }
}
