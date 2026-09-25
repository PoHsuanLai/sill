//! Coherence rules 1 and 2 (quire CONSUMING.md §5) for the banners and the notification card:
//! both stylesheets under `Profile::Strict`, and the real column through the markup lint, idle
//! (an SSR pass) and showing an expanded, grouped card with its actions (the harness).

mod banners_support;

use std::time::Duration;

use banners_support::{fake, live, note, start};
use dioxus::prelude::*;
use ds::lint::{LintConfig, Profile, Rule, assert_clean, markup};
use sill_settings::{SettingsFeed, ShellFile};
use sill_surfaces::Banners;
use sill_surfaces::surfaces::banners::{BANNER_VARS, BANNERS_CSS, BannerLink, BannersProps};
use sill_surfaces::surfaces::notification_card::{CARD_CSS, CARD_VARS};

fn strict() -> LintConfig {
    LintConfig {
        profile: Profile::Strict,
        own_vars: CARD_VARS
            .iter()
            .chain(BANNER_VARS.iter())
            .map(|v| (*v).to_owned())
            .collect(),
        ..LintConfig::default()
    }
}

fn full_css() -> String {
    format!("{}\n{}\n{}", ds::stylesheet(), CARD_CSS, BANNERS_CSS)
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

/// A grouped card, hovered, with two actions.
fn shown_html() -> String {
    let mut running = start(ShellFile::default());
    running.show(vec![
        live(note(
            2,
            "mail",
            "Two",
            "Body",
            &[("reply", "Reply"), ("archive", "Archive")],
        )),
        live(note(1, "mail", "One", "Body", &[])),
    ]);
    running.harness.advance(Duration::from_millis(700));
    let centre = running.harness.centre(".sill-card").expect("a card"); // test-only
    running.harness.pointer_move(centre);
    running.harness.advance(Duration::from_millis(60));
    assert_eq!(running.expansion().as_deref(), Some("expanded"));
    running.harness.html()
}

#[test]
fn the_stylesheets_are_clean_under_strict() {
    assert_clean(CARD_CSS, &strict());
    assert_clean(BANNERS_CSS, &strict());
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
