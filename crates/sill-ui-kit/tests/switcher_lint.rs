//! Coherence rules 1 and 2 for the switcher: its stylesheet under
//! `Profile::Strict`, and the real `SwitcherPanel` through the markup lint, hidden (an SSR pass)
//! and shown (the harness, past the show delay).

mod switcher_support;

use std::time::Duration;

use dioxus::prelude::*;
use ds::lint::{LintConfig, Profile, Rule, assert_clean, markup};
use sill_settings::{SettingsFeed, ShellFile};
use sill_ui_kit::SwitcherPanel;
use sill_ui_kit::links::{Dir, SwitcherAsk, SwitcherLink};
use sill_ui_kit::surfaces::switcher::SwitcherPanelProps;
use switcher_support::{fake, start, windows};

const SWITCHER_CSS: &str = include_str!("../src/style/switcher.css");

fn strict() -> LintConfig {
    LintConfig {
        profile: Profile::Strict,
        ..LintConfig::new(&ds::kits())
    }
}

fn full_css() -> String {
    format!("{}\n{}", ds::stylesheet(), SWITCHER_CSS)
}

/// `SwitcherPanel` on the fake services, rendered to HTML inside a runtime.
fn render_ssr() -> String {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .expect("a runtime builds"); // test-only
    let _entered = rt.enter();
    let (services, _feeds) = fake::services();
    let mut dom = VirtualDom::new_with_props(
        SwitcherPanel,
        SwitcherPanelProps {
            output: fake::output(),
        },
    );
    dom.provide_root_context(services);
    dom.provide_root_context(SettingsFeed::fixed(ShellFile::default()));
    dom.provide_root_context(SwitcherLink::new());
    dom.rebuild_in_place();
    dioxus_ssr::render(&dom)
}

/// The panel shown on three apps, from the harness.
fn shown_html() -> String {
    let mut running = start(
        ShellFile::default(),
        windows(&[("term", 1), ("calc", 2), ("edit", 3)]),
    );
    running.harness.advance(Duration::from_millis(40));
    running.ask(SwitcherAsk::Chord(Dir::Next));
    running.harness.advance(Duration::from_millis(260));
    assert_eq!(running.harness.count(".ds-switcher"), 1);
    running.harness.html()
}

#[test]
fn the_switcher_stylesheet_is_clean_under_strict() {
    assert_clean(SWITCHER_CSS, &strict());
}

#[test]
fn the_hidden_switcher_markup_is_clean() {
    let offences = markup(&render_ssr(), &full_css(), &strict());
    assert!(offences.is_empty(), "{offences:#?}");
}

#[test]
fn the_shown_switcher_markup_is_clean_and_has_no_raw_markup() {
    let offences = markup(&shown_html(), &full_css(), &strict());
    assert!(
        offences.iter().all(|o| o.rule != Rule::RawMarkup),
        "{offences:#?}"
    );
    assert!(offences.is_empty(), "{offences:#?}");
}
