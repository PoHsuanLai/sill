//! Coherence rules 1 and 2 (quire CONSUMING.md §5) for the notification center: its stylesheet
//! under `Profile::Strict` with no keyframes of its own, and the real `NotificationCenter`
//! through the markup lint, closed (an SSR pass) and open over a history (the harness).

mod center_support;

use center_support::{fake, opened, two_apps};
use dioxus::prelude::*;
use ds::lint::{LintConfig, Profile, Rule, assert_clean, markup};
use sill_settings::{SettingsFeed, ShellFile};
use sill_surfaces::NotificationCenter;
use sill_surfaces::surfaces::notification_center::{
    CenterLink, NOTIFICATION_CENTER_CSS, NotificationCenterProps,
};

fn strict() -> LintConfig {
    LintConfig {
        profile: Profile::Strict,
        ..LintConfig::default()
    }
}

fn full_css() -> String {
    // What the panel injects: its own sheet and the widgets' (M10 lane b).
    format!(
        "{}\n{}\n{}",
        ds::stylesheet(),
        NOTIFICATION_CENTER_CSS,
        sill_surfaces::widgets::widgets_css()
    )
}

/// The center on the fake services, closed, rendered to HTML inside a runtime.
fn render_ssr() -> String {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .expect("a runtime builds"); // test-only
    let _entered = rt.enter();
    let (services, _feeds) = fake::services();
    let mut dom = VirtualDom::new_with_props(
        NotificationCenter,
        NotificationCenterProps {
            output: fake::output(),
        },
    );
    dom.provide_root_context(services);
    dom.provide_root_context(SettingsFeed::fixed(ShellFile::default()));
    dom.provide_root_context(CenterLink::new());
    dom.rebuild_in_place();
    dioxus_ssr::render(&dom)
}

#[test]
fn the_center_stylesheet_is_clean_under_strict() {
    assert_clean(NOTIFICATION_CENTER_CSS, &strict());
}

#[test]
fn the_center_stylesheet_names_no_keyframes_of_its_own() {
    assert!(
        !NOTIFICATION_CENTER_CSS.contains("@keyframes"),
        "motion is quire's (coherence rule 4)"
    );
}

#[test]
fn the_closed_center_markup_is_clean() {
    let offences = markup(&render_ssr(), &full_css(), &strict());
    assert!(offences.is_empty(), "{offences:#?}");
}

#[test]
fn the_open_center_markup_is_clean_and_has_no_raw_markup() {
    let running = opened(two_apps());
    let offences = markup(&running.harness.html(), &full_css(), &strict());
    assert!(
        offences.iter().all(|o| o.rule != Rule::RawMarkup),
        "{offences:#?}"
    );
    assert!(offences.is_empty(), "{offences:#?}");
}
