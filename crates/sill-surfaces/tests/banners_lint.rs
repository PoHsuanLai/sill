//! Coherence rules 1 and 2 for the banners: the column's stylesheet under
//! `Profile::Strict` (the card has none of its own now: it is quire's), and the real column
//! through the markup lint, idle (an SSR pass) and showing a hovered, grouped card with its
//! actions and a rich body (the harness).

mod banners_support;

use std::time::Duration;

use banners_support::{fake, live, note, start};
use dioxus::prelude::*;
use ds::lint::{LintConfig, Profile, Rule, assert_clean, markup};
use sill_model::notifications::parse_body;
use sill_settings::{SettingsFeed, ShellFile};
use sill_surfaces::Banners;
use sill_surfaces::links::BannerLink;
use sill_surfaces::surfaces::banners::{BANNER_VARS, BANNERS_CSS, BannersProps};

fn strict() -> LintConfig {
    LintConfig {
        profile: Profile::Strict,
        own_vars: BANNER_VARS.iter().map(|v| (*v).to_owned()).collect(),
        ..LintConfig::new(&ds::kits())
    }
}

fn full_css() -> String {
    format!("{}\n{}", ds::stylesheet(), BANNERS_CSS)
}

/// The idle column, rendered to HTML inside a runtime.
fn render_ssr() -> String {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .expect("a runtime builds"); // test-only
    let _entered = rt.enter();
    let (services, _feeds) = fake::services();
    let mut dom = VirtualDom::new_with_props(
        Banners,
        BannersProps {
            output: fake::output(),
        },
    );
    dom.provide_root_context(services);
    dom.provide_root_context(SettingsFeed::fixed(ShellFile::default()));
    dom.provide_root_context(BannerLink::new());
    dom.rebuild_in_place();
    dioxus_ssr::render(&dom)
}

/// A grouped card, hovered, with two actions and a body with a link.
fn shown_html() -> String {
    let mut running = start(ShellFile::default());
    let mut newest = note(
        2,
        "mail",
        "Two",
        "",
        &[("reply", "Reply"), ("archive", "Archive")],
    );
    newest.body = parse_body("<b>Body</b> with <a href=\"https://x.org\">a link</a>");
    running.show(vec![
        live(newest),
        live(note(1, "mail", "One", "Body", &[])),
    ]);
    running.harness.advance(Duration::from_millis(700));
    let plate = running.plate();
    running.harness.pointer_move(plate);
    running.harness.advance(Duration::from_millis(60));
    assert_eq!(running.hover().as_deref(), Some("on"));
    running.harness.html()
}

#[test]
fn the_stylesheet_is_clean_under_strict() {
    assert_clean(BANNERS_CSS, &strict());
}

#[test]
fn the_stylesheet_names_no_motion_of_its_own() {
    for motion in ["@keyframes", "transition", "animation"] {
        assert!(
            !BANNERS_CSS.contains(motion),
            "{motion}: the stack's motion is quire's"
        );
    }
}

#[test]
fn the_idle_column_markup_is_clean() {
    let offences = markup(&render_ssr(), &full_css(), &strict());
    assert!(offences.is_empty(), "{offences:#?}");
}

#[test]
fn a_shown_card_is_quire_parts_only() {
    let offences = markup(&shown_html(), &full_css(), &strict());
    assert!(
        offences.iter().all(|o| o.rule != Rule::RawMarkup),
        "{offences:#?}"
    );
    assert!(offences.is_empty(), "{offences:#?}");
}
