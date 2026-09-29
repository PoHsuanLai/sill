//! Coherence rules 1 and 2 for the OSD: its stylesheet under
//! `Profile::Strict`, and the real `Osd` through the markup lint, hidden (an SSR pass) and
//! shown (the harness, after a change).

mod osd_support;

use std::time::Duration;

use dioxus::prelude::*;
use ds::lint::{LintConfig, Profile, Rule, assert_clean, markup};
use osd_support::{fake, look, start};
use sill_settings::{SettingsFeed, ShellFile};
use sill_ui_kit::Osd;
use sill_ui_kit::links::{LevelSource, OsdKind, OsdLink};
use sill_ui_kit::surfaces::osd::OsdProps;

const OSD_CSS: &str = include_str!("../src/style/osd.css");

fn strict() -> LintConfig {
    LintConfig {
        profile: Profile::Strict,
        ..LintConfig::new(&ds::kits())
    }
}

fn full_css() -> String {
    format!("{}\n{}", ds::stylesheet(), OSD_CSS)
}

/// `Osd` on the fake services, rendered to HTML inside a runtime (the appearance hook watches
/// files and the portal on it).
fn render_ssr() -> String {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .expect("a runtime builds"); // test-only
    let _entered = rt.enter();
    let (services, _feeds) = fake::services();
    let mut dom = VirtualDom::new_with_props(
        Osd,
        OsdProps {
            output: fake::output(),
        },
    );
    dom.provide_root_context(services);
    dom.provide_root_context(SettingsFeed::fixed(ShellFile::default()));
    dom.provide_root_context(OsdLink::new());
    dom.rebuild_in_place();
    dioxus_ssr::render(&dom)
}

/// The indicator shown for a volume change, from the harness.
fn shown_html() -> String {
    let mut running = start(ShellFile::default());
    running
        .link
        .show(OsdKind::Volume, vec![fake::output().id], LevelSource::Live);
    running.harness.advance(Duration::from_millis(60));
    assert_eq!(look(&running.harness).as_deref(), Some("entering"));
    running.harness.html()
}

#[test]
fn the_osd_stylesheet_is_clean_under_strict() {
    assert_clean(OSD_CSS, &strict());
}

#[test]
fn the_hidden_osd_markup_is_clean() {
    let offences = markup(&render_ssr(), &full_css(), &strict());
    assert!(offences.is_empty(), "{offences:#?}");
}

#[test]
fn the_shown_osd_markup_is_clean_and_has_no_raw_markup() {
    let offences = markup(&shown_html(), &full_css(), &strict());
    assert!(
        offences.iter().all(|o| o.rule != Rule::RawMarkup),
        "{offences:#?}"
    );
    assert!(offences.is_empty(), "{offences:#?}");
}
