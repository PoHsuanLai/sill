//! Coherence rules 1 and 2 (quire CONSUMING.md §5) for the bar: its stylesheet under
//! `Profile::Strict`, and an SSR render of the real `Bar` through the markup lint.

mod support;

use dioxus::prelude::*;
use ds::lint::{LintConfig, Profile, Rule, assert_clean, markup};
use sill_settings::{SettingsFeed, ShellFile};
use sill_surfaces::Bar;
use sill_surfaces::surfaces::bar::BarProps;

const BAR_CSS: &str = include_str!("../src/style/bar.css");

fn strict() -> LintConfig {
    LintConfig {
        profile: Profile::Strict,
        ..LintConfig::default()
    }
}

/// The bar on the fake services, rendered to HTML inside a runtime (the appearance hook
/// watches files and the portal on it).
fn render_ssr() -> String {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .expect("a runtime builds"); // test-only
    let _entered = rt.enter();
    let (services, _feeds) = support::fake::services();
    let mut dom = VirtualDom::new_with_props(
        Bar,
        BarProps {
            output: support::fake::output(),
        },
    );
    dom.provide_root_context(services);
    dom.provide_root_context(SettingsFeed::fixed(ShellFile::default()));
    dom.rebuild_in_place();
    dioxus_ssr::render(&dom)
}

fn full_css() -> String {
    format!("{}\n{}", ds::stylesheet(), BAR_CSS)
}

#[test]
fn the_bar_stylesheet_is_clean_under_strict() {
    assert_clean(BAR_CSS, &strict());
}

#[test]
fn the_bar_markup_lint_is_clean() {
    let offences = markup(&render_ssr(), &full_css(), &strict());
    assert!(offences.is_empty(), "{offences:#?}");
}

#[test]
fn the_bar_renders_no_raw_markup() {
    let offences = markup(&render_ssr(), &full_css(), &strict());
    assert!(
        offences.iter().all(|o| o.rule != Rule::RawMarkup),
        "{offences:#?}"
    );
}
