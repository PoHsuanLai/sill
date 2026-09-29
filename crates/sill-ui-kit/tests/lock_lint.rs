//! Coherence rules 1 and 2 for the lock screen: its stylesheet under
//! `Profile::Strict`, and the real `LockScreen` markup (with the prompt, and without) through
//! the markup lint.

use sill_ui_kit::harness::fake;

use dioxus::prelude::*;
use ds::lint::{LintConfig, Profile, assert_clean, markup};
use sill_settings::{SettingsFeed, ShellFile};
use sill_ui_kit::LockScreen;
use sill_ui_kit::links::{LockLink, PromptOutput};
use sill_ui_kit::surfaces::lock::LockScreenProps;

const LOCK_CSS: &str = include_str!("../src/style/lock.css");

fn strict() -> LintConfig {
    LintConfig {
        profile: Profile::Strict,
        ..LintConfig::new(&ds::kits())
    }
}

fn full_css() -> String {
    format!("{}\n{}", ds::stylesheet(), LOCK_CSS)
}

fn render_ssr(prompt: PromptOutput) -> String {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .expect("a runtime builds"); // test-only
    let _entered = rt.enter();
    let (services, _feeds) = fake::services();
    let link = LockLink::new();
    link.set(prompt);
    let mut dom = VirtualDom::new_with_props(
        LockScreen,
        LockScreenProps {
            output: fake::output(),
        },
    );
    dom.provide_root_context(services);
    dom.provide_root_context(SettingsFeed::fixed(ShellFile::default()));
    dom.provide_root_context(link);
    dom.rebuild_in_place();
    dioxus_ssr::render(&dom)
}

#[test]
fn the_lock_stylesheet_is_clean_under_strict() {
    assert_clean(LOCK_CSS, &strict());
}

#[test]
fn the_lock_markup_is_clean_with_and_without_the_prompt() {
    let with = render_ssr(PromptOutput::On(fake::output().id));
    let without = render_ssr(PromptOutput::Nowhere);
    for html in [with, without] {
        let offences = markup(&html, &full_css(), &strict());
        assert!(offences.is_empty(), "{offences:#?}");
    }
}
