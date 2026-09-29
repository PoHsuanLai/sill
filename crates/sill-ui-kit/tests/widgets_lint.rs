//! Coherence rules 1, 2 and 4 for the widgets: sill's widgets' stylesheets, the desktop
//! layer's and Edit Widgets' under
//! `Profile::Strict` with no keyframes of their own, and the markup of every widget in both
//! hosts, of the desktop layer and of Edit Widgets through the markup lint.

use sill_ui_kit::harness::fake;

use std::cell::RefCell;
use std::time::Duration;

use dioxus::prelude::*;
use ds::lint::{LintConfig, Profile, assert_clean, markup};
use ds::{Ds, Material, RootExtent};
use ds_native::{Clock, Harness, HarnessConfig, Viewport};
use sill_settings::{SettingsFeed, ShellFile, WidgetKind};
use sill_ui_kit::harness::SurfaceHarness;
use sill_ui_kit::hooks::PinnedAppearance;
use sill_ui_kit::links::{EditWidgetsLink, PlacementFile};
use sill_ui_kit::services::Services;
use sill_ui_kit::surfaces::desktop_widgets::DESKTOP_WIDGETS_CSS;
use sill_ui_kit::surfaces::edit_widgets::EDIT_WIDGETS_CSS;
use sill_ui_kit::widgets::{Widget, WidgetHost, widgets_css};
use sill_ui_kit::{DesktopWidgets, EditWidgets};

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
        ..LintConfig::new(&ds::kits())
    }
}

fn full_css() -> String {
    format!(
        "{}\n{}\n{}\n{}",
        ds::stylesheet(),
        widgets_css(),
        DESKTOP_WIDGETS_CSS,
        EDIT_WIDGETS_CSS
    )
}

/// What the harness draws.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Drawn {
    /// Every widget in one host.
    Host(WidgetHost),
    /// The desktop layer.
    Layer,
    /// Edit Widgets, open.
    Edit,
}

#[derive(Clone)]
struct World {
    services: Services,
    settings: SettingsFeed,
    host: WidgetHost,
}

thread_local! {
    static WORLD: RefCell<Option<World>> = const { RefCell::new(None) };
}

/// Every widget in one host, the desktop layer, or Edit Widgets.
fn app() -> Element {
    let world = WORLD.with(|w| w.borrow().clone());
    let Some(world) = world else {
        return rsx! {};
    };
    use_context_provider(|| world.services.clone());
    use_context_provider(PinnedAppearance::default);
    use_context_provider(|| world.settings.clone());
    let host = world.host;
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

const VIEWPORT: Viewport = Viewport {
    width: 1280,
    height: 800,
    scale_percent: 100,
};

/// What keeps a document's services and links from closing while it draws.
type Alive = Box<dyn std::any::Any>;

/// Every widget on `settings`, the way the layer and the gallery are asked to draw them.
fn settings() -> ShellFile {
    let mut file = ShellFile::default();
    file.widgets.world_clocks = "Europe/London".to_owned();
    file.widgets.desktop_widgets = ALL.to_vec();
    file
}

fn html_of(drawn: Drawn) -> String {
    // What the document is fed through stays alive until it has been drawn.
    let (mut harness, _alive) = match drawn {
        Drawn::Host(host) => hosted(host),
        Drawn::Layer => layer(),
        Drawn::Edit => edit(),
    };
    for _ in 0..10 {
        harness.advance(Duration::from_millis(40));
    }
    harness.html()
}

/// Every widget in `host`, on a `Ds` root.
fn hosted(host: WidgetHost) -> (Harness, Alive) {
    let (services, feeds) = fake::services();
    feeds.media.send_replace(fake::media());
    WORLD.with(|w| {
        *w.borrow_mut() = Some(World {
            services,
            settings: SettingsFeed::fixed(settings()),
            host,
        });
    });
    let harness =
        Harness::with_config(app, HarnessConfig::new(VIEWPORT).with_clock(Clock::Virtual));
    (harness, Box::new(feeds))
}

/// The desktop layer, with its places in a file that does not exist.
fn layer() -> (Harness, Alive) {
    let places = PlacementFile(
        std::env::temp_dir()
            .join(format!("sill-widgets-lint-{}", std::process::id()))
            .join("desktop-widgets.json"),
    );
    let run = SurfaceHarness::<DesktopWidgets>::setup(places, ())
        .settings(settings())
        .viewport(VIEWPORT)
        .start();
    run.feeds.media.send_replace(fake::media());
    (run.harness, Box::new((run.feeds, run.link)))
}

/// Edit Widgets, open.
fn edit() -> (Harness, Alive) {
    let run = SurfaceHarness::<EditWidgets>::setup(EditWidgetsLink::new(), ())
        .settings(settings())
        .viewport(VIEWPORT)
        .start();
    run.feeds.media.send_replace(fake::media());
    run.link.open(fake::output().id);
    (run.harness, Box::new((run.feeds, run.link)))
}

#[test]
fn the_widget_stylesheets_are_clean_under_strict() {
    assert_clean(&widgets_css(), &strict());
    assert_clean(DESKTOP_WIDGETS_CSS, &strict());
    assert_clean(EDIT_WIDGETS_CSS, &strict());
}

#[test]
fn the_widget_stylesheets_name_no_keyframes_of_their_own() {
    assert!(!widgets_css().contains("@keyframes"));
    assert!(!DESKTOP_WIDGETS_CSS.contains("@keyframes"));
    assert!(!EDIT_WIDGETS_CSS.contains("@keyframes"));
}

#[test]
fn every_widget_is_clean_markup_in_both_hosts_on_the_layer_and_in_edit_widgets() {
    let drawn = [
        (Drawn::Host(WidgetHost::Tile), "widget-battery"),
        (Drawn::Host(WidgetHost::Desktop), "widget-battery"),
        (Drawn::Layer, "widget-battery"),
        (Drawn::Edit, "ds-widget-gallery"),
    ];
    for (drawn, mark) in drawn {
        let html = html_of(drawn);
        assert!(html.contains(mark), "{drawn:?} drew {mark}");
        let offences = markup(&html, &full_css(), &strict());
        assert!(offences.is_empty(), "{drawn:?}: {offences:#?}");
    }
}
