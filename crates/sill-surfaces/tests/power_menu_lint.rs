//! Coherence rules 1 and 2 for the power menu: its stylesheet under
//! `Profile::Strict` (no raw colour, duration, spacing, radius, font size or keyframes), and
//! the real `PowerMenu` through the markup lint, hidden (an SSR pass) and open (the harness).

mod power_menu_support;

use dioxus::prelude::*;
use ds::lint::{LintConfig, Profile, Rule, assert_clean, markup};
use power_menu_support::{fake, opened};
use sill_model::session::Session;
use sill_settings::{SettingsFeed, ShellFile};
use sill_surfaces::PowerMenu;
use sill_surfaces::surfaces::power_menu::{
    DryRunSession, PowerMenuBackend, PowerMenuLink, PowerMenuProps, Preselect,
};

const POWER_MENU_CSS: &str = include_str!("../src/style/power_menu.css");

fn strict() -> LintConfig {
    LintConfig {
        profile: Profile::Strict,
        ..LintConfig::default()
    }
}

fn full_css() -> String {
    format!("{}\n{}", ds::stylesheet(), POWER_MENU_CSS)
}

/// `PowerMenu` on the fake services, hidden, rendered to HTML inside a runtime (the appearance
/// hook watches files and the portal on it).
fn render_ssr() -> String {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .expect("a runtime builds"); // test-only
    let _entered = rt.enter();
    let (services, _feeds) = fake::services();
    let mut dom = VirtualDom::new_with_props(
        PowerMenu,
        PowerMenuProps {
            output: fake::output(),
        },
    );
    dom.provide_root_context(services);
    dom.provide_root_context(SettingsFeed::fixed(ShellFile::default()));
    dom.provide_root_context(PowerMenuLink::new());
    dom.provide_root_context(PowerMenuBackend::dry_run(DryRunSession::new(
        Session::everything(),
        None,
    )));
    dom.rebuild_in_place();
    dioxus_ssr::render(&dom)
}

#[test]
fn the_power_menu_stylesheet_is_clean_under_strict() {
    assert_clean(POWER_MENU_CSS, &strict());
}

#[test]
fn the_power_menu_stylesheet_names_no_keyframes_of_its_own() {
    assert!(
        !POWER_MENU_CSS.contains("@keyframes"),
        "motion is quire's (coherence rule 4)"
    );
}

#[test]
fn the_hidden_power_menu_markup_is_clean() {
    let offences = markup(&render_ssr(), &full_css(), &strict());
    assert!(offences.is_empty(), "{offences:#?}");
}

#[test]
fn the_open_power_menu_markup_is_clean_and_has_no_raw_markup() {
    let running = opened(
        ShellFile::default(),
        Session::everything(),
        Preselect::Default,
    );
    let offences = markup(&running.harness.html(), &full_css(), &strict());
    assert!(
        offences.iter().all(|o| o.rule != Rule::RawMarkup),
        "{offences:#?}"
    );
    assert!(offences.is_empty(), "{offences:#?}");
}
