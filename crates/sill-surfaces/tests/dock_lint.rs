//! Coherence rules 1 and 2 for the dock: its stylesheets under
//! `Profile::Strict`, and an SSR render of the real `Dock` through the markup lint.

mod dock_support;

use dioxus::prelude::*;
use ds::lint::{LintConfig, Profile, Rule, assert_clean, markup};
use sill_settings::{SettingsFeed, ShellFile};
use sill_surfaces::Dock;
use sill_surfaces::surfaces::dock::DockProps;

const DOCK_CSS: &str = include_str!("../src/style/dock.css");
const POPUP_CSS: &str = include_str!("../src/style/dock_popup.css");

fn strict() -> LintConfig {
    LintConfig {
        profile: Profile::Strict,
        ..LintConfig::new(&ds::kits())
    }
}

/// The dock on the fake services, rendered to HTML inside a runtime (the appearance hook
/// watches files and the portal on it).
fn render_ssr() -> String {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .expect("a runtime builds"); // test-only
    let _entered = rt.enter();
    let (services, _feeds) = dock_support::services();
    let mut dom = VirtualDom::new_with_props(
        Dock,
        DockProps {
            output: dock_support::fake::output(),
        },
    );
    dom.provide_root_context(services);
    dom.provide_root_context(SettingsFeed::fixed(ShellFile::default()));
    dom.rebuild_in_place();
    dioxus_ssr::render(&dom)
}

fn full_css() -> String {
    format!("{}\n{}", ds::stylesheet(), DOCK_CSS)
}

#[test]
fn the_dock_stylesheet_is_clean_under_strict() {
    assert_clean(DOCK_CSS, &strict());
}

#[test]
fn the_popup_stylesheet_is_clean_under_strict() {
    assert_clean(POPUP_CSS, &strict());
}

#[test]
fn the_dock_markup_lint_is_clean() {
    let offences = markup(&render_ssr(), &full_css(), &strict());
    assert!(offences.is_empty(), "{offences:#?}");
}

#[test]
fn the_dock_renders_no_raw_markup() {
    let offences = markup(&render_ssr(), &full_css(), &strict());
    assert!(
        offences.iter().all(|o| o.rule != Rule::RawMarkup),
        "{offences:#?}"
    );
}
