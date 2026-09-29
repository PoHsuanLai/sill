//! Coherence rules 1 and 2 for the launcher: its stylesheet under
//! `Profile::Strict`, and renders of the real `Launcher` (open over results, and with its
//! actions menu) and `LauncherCatcher` through the markup lint. The panel is rendered by the
//! harness (its palette embedded, measured and focused as live) and by an SSR pass.

mod launcher_harness;

use launcher_harness::launcher_support;

use dioxus::prelude::*;
use ds::lint::{LintConfig, Profile, Rule, assert_clean, markup};
use sill_settings::{SettingsFeed, ShellFile};
use sill_surfaces::links::LauncherLink;
use sill_surfaces::surfaces::launcher::{LauncherCatcherProps, LauncherProps};
use sill_surfaces::{Launcher, LauncherCatcher};

const LAUNCHER_CSS: &str = include_str!("../src/style/launcher.css");

fn strict() -> LintConfig {
    LintConfig {
        profile: Profile::Strict,
        ..LintConfig::new(&ds::kits())
    }
}

/// `root` on the fake services with the launcher open on the test output over "fi"'s results,
/// rendered to HTML inside a runtime (the appearance hook watches files and the portal on it).
fn render_ssr(root: VirtualDom) -> String {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .expect("a runtime builds"); // test-only
    let _entered = rt.enter();
    let (services, feeds) = launcher_support::services();
    feeds.results.send_replace(sill_model::results::Results {
        generation: sill_launcher::Generation(1),
        items: launcher_support::search("fi"),
        sections: Vec::new(),
    });
    let link = LauncherLink::new();
    link.open(launcher_support::fake::output().id, "fi".to_owned());
    let mut dom = root;
    dom.provide_root_context(services);
    dom.provide_root_context(SettingsFeed::fixed(ShellFile::default()));
    dom.provide_root_context(link);
    dom.provide_root_context(launcher_support::providers());
    dom.rebuild_in_place();
    dioxus_ssr::render(&dom)
}

fn panel_html() -> String {
    launcher_harness::open("fi").harness.html()
}

/// The SSR pass of the panel: its root and the Ds, the palette not yet mounted.
fn panel_ssr() -> String {
    render_ssr(VirtualDom::new_with_props(
        Launcher,
        LauncherProps {
            output: launcher_support::fake::output(),
        },
    ))
}

fn catcher_html() -> String {
    render_ssr(VirtualDom::new_with_props(
        LauncherCatcher,
        LauncherCatcherProps {
            output: launcher_support::fake::output(),
        },
    ))
}

fn full_css() -> String {
    format!("{}\n{}", ds::stylesheet(), LAUNCHER_CSS)
}

#[test]
fn the_launcher_stylesheet_is_clean_under_strict() {
    assert_clean(LAUNCHER_CSS, &strict());
}

#[test]
fn the_panel_renders_the_palette() {
    let html = panel_html();
    assert!(html.contains("class=\"ds-palette\""), "{html}");
    assert!(html.contains("id=\"launcher-card\""), "the blur element");
    let ssr = panel_ssr();
    assert!(ssr.contains("id=\"panel\""), "{ssr}");
}

#[test]
fn the_panel_markup_lint_is_clean() {
    let offences = markup(&panel_html(), &full_css(), &strict());
    assert!(offences.is_empty(), "{offences:#?}");
}

#[test]
fn the_panel_renders_no_raw_markup() {
    let offences = markup(&panel_html(), &full_css(), &strict());
    assert!(
        offences.iter().all(|o| o.rule != Rule::RawMarkup),
        "{offences:#?}"
    );
}

#[test]
fn the_panel_with_its_preview_pane_and_sections_lints_clean() {
    let mut open = launcher_harness::open("m9");
    open.link.press(sill_surfaces::links::LinkKey::QuickLook);
    open.harness.advance(std::time::Duration::from_millis(60));
    let html = open.harness.html();
    assert!(html.contains("ds-preview"), "the pane is up");
    let offences = markup(&html, &full_css(), &strict());
    assert!(offences.is_empty(), "{offences:#?}");
}

#[test]
fn the_catcher_markup_lint_is_clean() {
    let html = catcher_html();
    assert!(html.contains("sill-launcher-catcher"), "{html}");
    let offences = markup(&html, &full_css(), &strict());
    assert!(offences.is_empty(), "{offences:#?}");
}

#[test]
fn the_panel_ssr_markup_lint_is_clean() {
    let offences = markup(&panel_ssr(), &full_css(), &strict());
    assert!(offences.is_empty(), "{offences:#?}");
}
