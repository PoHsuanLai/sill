//! Coherence rules 1 and 2 (quire CONSUMING.md §5) for the wallpaper: its stylesheet under
//! `Profile::Strict`, and an SSR render of the real `Wallpaper` through the markup lint, with
//! and without a prepared image.

mod support;

use dioxus::prelude::*;
use ds::lint::{LintConfig, Profile, Rule, assert_clean, markup};
use sill_services::{OutputName, WallpaperImage, Wallpapers};
use sill_surfaces::Wallpaper;
use sill_surfaces::surfaces::wallpaper::WallpaperProps;
use tokio::sync::watch;

const WALLPAPER_CSS: &str = include_str!("../src/style/wallpaper.css");

fn strict() -> LintConfig {
    LintConfig {
        profile: Profile::Strict,
        ..LintConfig::default()
    }
}

/// `Wallpaper` on the fake services with `wallpapers` replaced by `fed`, rendered to HTML
/// inside a runtime (the appearance hook watches files and the portal on it).
fn render_ssr(fed: Wallpapers) -> String {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .expect("a runtime builds"); // test-only
    let _entered = rt.enter();
    let (mut services, _feeds) = support::fake::services();
    let (_tx, rx) = watch::channel(fed);
    services.wallpapers = rx;
    let mut dom = VirtualDom::new_with_props(
        Wallpaper,
        WallpaperProps {
            output: support::fake::output(),
        },
    );
    dom.provide_root_context(services);
    dom.rebuild_in_place();
    dioxus_ssr::render(&dom)
}

fn full_css() -> String {
    format!("{}\n{}", ds::stylesheet(), WALLPAPER_CSS)
}

#[test]
fn the_wallpaper_stylesheet_is_clean_under_strict() {
    assert_clean(WALLPAPER_CSS, &strict());
}

#[test]
fn the_wallpaper_markup_lint_is_clean_with_no_image() {
    let offences = markup(&render_ssr(Wallpapers::default()), &full_css(), &strict());
    assert!(offences.is_empty(), "{offences:#?}");
}

#[test]
fn the_wallpaper_markup_lint_is_clean_with_an_image() {
    let fed = Wallpapers {
        outputs: vec![WallpaperImage {
            output: OutputName("DP-1".to_owned()),
            light: "/tmp/sill-wallpaper-lint/light.png".into(),
            dark: "/tmp/sill-wallpaper-lint/dark.png".into(),
        }],
    };
    let offences = markup(&render_ssr(fed), &full_css(), &strict());
    assert!(offences.is_empty(), "{offences:#?}");
}

#[test]
fn the_wallpaper_renders_no_raw_markup() {
    let fed = Wallpapers {
        outputs: vec![WallpaperImage {
            output: OutputName("DP-1".to_owned()),
            light: "/tmp/sill-wallpaper-lint/light.png".into(),
            dark: "/tmp/sill-wallpaper-lint/dark.png".into(),
        }],
    };
    let offences = markup(&render_ssr(fed), &full_css(), &strict());
    assert!(
        offences.iter().all(|o| o.rule != Rule::RawMarkup),
        "{offences:#?}"
    );
}
