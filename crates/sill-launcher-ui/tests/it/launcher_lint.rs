//! Coherence rules 1 and 2 for the launcher's panel over results (the empty panel and the
//! catcher are rows of `crates/sill/tests/lint.rs`): the real `Launcher` (open over results, and
//! with its preview pane) through the markup lint, rendered by the harness (its palette
//! embedded, measured and focused as live) and by an SSR pass.

use crate::launcher_harness;

use ds_harness::Driver;
use launcher_harness::launcher_support;

use dioxus::prelude::*;
use ds_lint::{LintConfig, Profile, Rule, markup};
use sill_launcher_ui::Launcher;
use sill_settings::ShellFile;
use sill_ui_kit::links::LauncherLink;
use sill_ui_kit::settings::SettingsFeed;
use sill_ui_kit::surface::{MountProps, mount};

const LAUNCHER_CSS: &str = include_str!("../../src/launcher/style.css");

fn strict() -> LintConfig {
    LintConfig {
        profile: Profile::Strict,
        ..LintConfig::new(&sill_shell_kit::kits())
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
        mount::<Launcher>,
        MountProps::<Launcher>::new(launcher_support::fake::output(), ()),
    ))
}

fn full_css() -> String {
    format!("{}\n{}", sill_shell_kit::stylesheet(), LAUNCHER_CSS)
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
    // The report's look is held (until `_held` drops), so the pane is the pending one however fast
    // the page would raster.
    let (look, _held) = launcher_harness::launcher_support::m9::gated(
        launcher_harness::launcher_support::m9::folder().join("m9-report.pdf"),
    );
    let mut open = launcher_harness::open_looking("m9", Some(look));
    open.press(launcher_harness::Press::QuickLook);
    open.harness.advance(std::time::Duration::from_millis(60));
    let html = open.harness.html();
    assert!(html.contains("ds-preview"), "the pane is up");
    let offences = markup(&html, &full_css(), &strict());
    assert!(offences.is_empty(), "{offences:#?}");
}

#[test]
fn the_panel_ssr_markup_lint_is_clean() {
    let offences = markup(&panel_ssr(), &full_css(), &strict());
    assert!(offences.is_empty(), "{offences:#?}");
}

#[test]
fn the_panel_in_companion_mode_with_an_answer_card_lints_clean() {
    use launcher_harness::launcher_support::companion::{companion, draft, fresh};
    let open = launcher_harness::open_in("", sill_launcher::LauncherMode::Companion, |bar| {
        bar.companion
            .send_replace(companion(vec![draft()], fresh()));
    });
    let html = open.harness.html();
    assert!(html.contains("ds-answer-card"), "the card is up: {html}");
    let offences = markup(&html, &full_css(), &strict());
    assert!(offences.is_empty(), "{offences:#?}");
}
