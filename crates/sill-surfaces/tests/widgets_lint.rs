//! Coherence rules 1, 2 and 4 (quire CONSUMING.md §5) for the widgets (M10): every widget's
//! stylesheet, the frame's and the desktop layer's under `Profile::Strict` with no keyframes of
//! their own, and the markup of every widget in both hosts and of the desktop layer through the
//! markup lint.

#[path = "support/fake.rs"]
mod fake;

use std::cell::RefCell;
use std::time::Duration;

use dioxus::prelude::*;
use ds::lint::{LintConfig, Profile, assert_clean, markup};
use ds::{Ds, Material, RootExtent};
use ds_native::{Harness, Viewport};
use sill_services::Services;
use sill_settings::{SettingsFeed, ShellFile, WidgetKind};
use sill_surfaces::DesktopWidgets;
use sill_surfaces::surfaces::desktop_widgets::DESKTOP_WIDGETS_CSS;
use sill_surfaces::widgets::{Widget, WidgetHost, widgets_css};

const ALL: [WidgetKind; 5] = [
    WidgetKind::Calendar,
    WidgetKind::UpNext,
    WidgetKind::Battery,
    WidgetKind::NowPlaying,
    WidgetKind::WorldClock,
];

fn strict() -> LintConfig {
    LintConfig {
        profile: Profile::Strict,
        ..LintConfig::default()
    }
}

fn full_css() -> String {
    format!(
        "{}\n{}\n{}",
        ds::stylesheet(),
        widgets_css(),
        DESKTOP_WIDGETS_CSS
    )
}

#[derive(Clone)]
struct World {
    services: Services,
    settings: SettingsFeed,
    host: Option<WidgetHost>,
}

thread_local! {
    static WORLD: RefCell<Option<World>> = const { RefCell::new(None) };
}

/// Every widget in one host, or the desktop layer when no host is named.
fn app() -> Element {
    let world = WORLD.with(|w| w.borrow().clone());
    let Some(world) = world else {
        return rsx! {};
    };
    use_context_provider(|| world.services.clone());
    use_context_provider(|| world.settings.clone());
    let Some(host) = world.host else {
        return rsx! { DesktopWidgets { output: fake::output() } };
    };
    let material = match host {
        WidgetHost::Tile => Material::Popover,
        WidgetHost::Desktop => Material::Widget,
    };
    rsx! {
        Ds { appearance: ds::Appearance::default(), material, extent: RootExtent::Viewport,
            style { {widgets_css()} }
            for kind in ALL {
                Widget { kind, host }
            }
        }
    }
}

fn html_of(host: Option<WidgetHost>) -> String {
    let (services, feeds) = fake::services();
    feeds.media.send_replace(fake::media());
    let mut file = ShellFile::default();
    file.widgets.world_clocks = "Europe/London".to_owned();
    file.widgets.desktop_widgets = ALL.to_vec();
    WORLD.with(|w| {
        *w.borrow_mut() = Some(World {
            services,
            settings: SettingsFeed::fixed(file),
            host,
        });
    });
    let viewport = Viewport {
        width: 1280,
        height: 800,
        scale_percent: 100,
    };
    let mut harness = Harness::new(app, viewport);
    for _ in 0..10 {
        harness.advance(Duration::from_millis(40));
    }
    harness.html()
}

#[test]
fn the_widget_stylesheets_are_clean_under_strict() {
    assert_clean(&widgets_css(), &strict());
    assert_clean(DESKTOP_WIDGETS_CSS, &strict());
}

#[test]
fn the_widget_stylesheets_name_no_keyframes_of_their_own() {
    assert!(!widgets_css().contains("@keyframes"));
    assert!(!DESKTOP_WIDGETS_CSS.contains("@keyframes"));
}

#[test]
fn every_widget_is_clean_markup_in_both_hosts_and_on_the_layer() {
    for host in [Some(WidgetHost::Tile), Some(WidgetHost::Desktop), None] {
        let html = html_of(host);
        assert!(html.contains("widget-battery"), "{host:?} drew the widgets");
        let offences = markup(&html, &full_css(), &strict());
        assert!(offences.is_empty(), "{host:?}: {offences:#?}");
    }
}
