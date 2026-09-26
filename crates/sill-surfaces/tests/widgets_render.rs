//! What the widgets draw (M10 lane b), each in a `ds_native::Harness` on the fake services: the
//! frame (quire's `WidgetFrame`: a tile in the notification center, the Widget material's card
//! on the desktop, each at its footprint), Battery's rings from the machine and the connected
//! devices and their bump on a change, Now Playing as the control center's module (its presses
//! reach the fake media queue, never a real player), World Clock's faces from
//! `widgets.world_clocks`; and the notification center's slot, which draws
//! `widgets.center` in order under the history.

mod center_support;

use center_support::fake;
use std::cell::RefCell;
use std::time::Duration;

use dioxus::prelude::*;
use ds::{Ds, Material, RootExtent};
use ds_native::{Harness, Viewport};
use sill_services::{MediaCommand, MediaVerb, Services};
use sill_settings::{SettingsFeed, ShellFile, WidgetKind};
use sill_surfaces::widgets::{Widget, WidgetHost, widgets_css};

/// What the harness's root provides.
#[derive(Clone)]
struct World {
    services: Services,
    settings: SettingsFeed,
    kind: WidgetKind,
    host: WidgetHost,
}

thread_local! {
    static WORLD: RefCell<Option<World>> = const { RefCell::new(None) };
}

fn app() -> Element {
    let world = WORLD.with(|w| w.borrow().clone());
    let Some(world) = world else {
        return rsx! {};
    };
    use_context_provider(|| world.services.clone());
    use_context_provider(|| world.settings.clone());
    let material = match world.host {
        WidgetHost::Tile => Material::Popover,
        WidgetHost::Desktop => Material::Widget,
    };
    rsx! {
        Ds { appearance: ds::Appearance::default(), material, extent: RootExtent::Viewport,
            style { {widgets_css()} }
            div { style: "width:344px;",
                Widget { kind: world.kind, host: world.host }
            }
        }
    }
}

const VIEWPORT: Viewport = Viewport {
    width: 480,
    height: 480,
    scale_percent: 100,
};

/// A running widget and its feeds.
struct Running {
    harness: Harness,
    feeds: fake::Feeds,
}

fn start(kind: WidgetKind, host: WidgetHost, file: ShellFile) -> Running {
    let (services, feeds) = fake::services();
    WORLD.with(|w| {
        *w.borrow_mut() = Some(World {
            services,
            settings: SettingsFeed::fixed(file),
            kind,
            host,
        });
    });
    let mut harness = Harness::new(app, VIEWPORT);
    harness.advance(Duration::from_millis(100));
    Running { harness, feeds }
}

#[test]
fn in_the_center_the_frame_is_quires_tile_with_the_title_row() {
    let running = start(WidgetKind::Battery, WidgetHost::Tile, ShellFile::default());
    let h = &running.harness;
    assert_eq!(h.count(".ds-widget#widget-battery"), 1);
    assert_eq!(
        h.attr("#widget-battery", "data-host").as_deref(),
        Some("tile")
    );
    assert_eq!(
        h.attr("#widget-battery", "data-size").as_deref(),
        Some("medium")
    );
    assert_eq!(
        h.text_of("#widget-battery .ds-widget-title-text")
            .as_deref(),
        Some("Batteries")
    );
    assert_eq!(
        h.count(".ds[*|data-material=widget]"),
        0,
        "no second material"
    );
    let tile = h.rect("#widget-battery").expect("the tile is laid out");
    assert_eq!(
        (tile.size.width.0.round(), tile.size.height.0.round()),
        (344.0, 164.0)
    );
}

#[test]
fn on_the_desktop_the_frame_is_a_surface_in_the_widget_material() {
    let running = start(
        WidgetKind::Battery,
        WidgetHost::Desktop,
        ShellFile::default(),
    );
    let h = &running.harness;
    assert_eq!(
        h.attr("#widget-battery", "data-host").as_deref(),
        Some("desktop")
    );
    assert_eq!(
        h.attr("#widget-battery", "data-size").as_deref(),
        Some("small")
    );
    assert_eq!(
        h.count(".ds[*|data-material=widget] > .ds-widget#widget-battery"),
        1,
        "the card is the Widget material's plate: {}",
        h.html()
    );
    let card = h.rect("#widget-battery").expect("the card is laid out");
    assert_eq!(
        (card.size.width.0.round(), card.size.height.0.round()),
        (164.0, 164.0)
    );
}

#[test]
fn battery_lists_the_machine_then_connected_devices_with_a_level() {
    let running = start(WidgetKind::Battery, WidgetHost::Tile, ShellFile::default());
    let h = &running.harness;
    // The fake machine is at 64%; Buds are connected at 80%; the keyboard is not connected.
    assert_eq!(h.count("#widget-battery .ds-battery"), 2);
    let levels: Vec<_> = (1..=2)
        .map(|n| {
            h.attr(
                &format!("#widget-battery .sill-widget-battery-cell:nth-child({n}) .ds-battery"),
                "aria-valuenow",
            )
        })
        .collect();
    assert_eq!(levels, vec![Some("64".to_owned()), Some("80".to_owned())]);
    let html = h.html();
    assert!(html.contains("64%"), "the machine's level");
    assert!(
        html.contains("Buds") && html.contains("80%"),
        "the headphones"
    );
    assert!(
        !html.contains("Keychron"),
        "a disconnected device is left out"
    );
}

#[test]
fn battery_says_so_when_there_is_none() {
    let mut running = start(WidgetKind::Battery, WidgetHost::Tile, ShellFile::default());
    running
        .feeds
        .battery
        .send_replace(sill_services::Battery::default());
    running
        .feeds
        .bluetooth
        .send_replace(sill_services::Bluetooth::default());
    running.harness.advance(Duration::from_millis(100));
    let h = &running.harness;
    assert_eq!(h.count("#widget-battery .ds-battery"), 0);
    assert_eq!(
        h.text_of("#widget-battery .sill-widget-quiet").as_deref(),
        Some("No batteries")
    );
}

#[test]
fn now_playing_is_the_control_centers_module_and_presses_reach_the_fake() {
    let mut running = start(
        WidgetKind::NowPlaying,
        WidgetHost::Tile,
        ShellFile::default(),
    );
    running.feeds.media.send_replace(fake::media());
    running.harness.advance(Duration::from_millis(100));
    let h = &running.harness;
    assert_eq!(
        h.text_of("#widget-now-playing .sill-cc-title").as_deref(),
        Some("So What")
    );
    assert_eq!(
        h.attr(
            "#widget-now-playing .sill-widget-now-playing .ds-module-panel",
            "data-plate"
        )
        .as_deref(),
        Some("bare"),
        "the module's own plate is the frame's"
    );
    let next = h
        .centre("#widget-now-playing .sill-cc-media-buttons .ds-icon-button:last-child")
        .expect("the Next button is laid out");
    running.harness.click(next);
    running.harness.advance(Duration::from_millis(50));
    let sent: Vec<_> =
        std::iter::from_fn(|| running.feeds.media_commands.try_recv().ok()).collect();
    assert!(
        sent.iter().any(|c| matches!(
            c,
            MediaCommand::Press {
                verb: MediaVerb::Next,
                ..
            }
        )),
        "{sent:?}"
    );
}

#[test]
fn world_clock_draws_a_row_per_known_city() {
    let mut file = ShellFile::default();
    file.widgets.world_clocks = "Europe/London; Nowhere/Atlantis; America/New_York".to_owned();
    let running = start(WidgetKind::WorldClock, WidgetHost::Tile, file);
    let h = &running.harness;
    assert_eq!(
        h.count("#widget-world-clock .ds-clock[*|data-look=digital]"),
        2
    );
    let html = h.html();
    assert!(
        html.contains("London") && html.contains("New York"),
        "{html}"
    );
    assert!(!html.contains("Atlantis"));
}

#[test]
fn world_clock_with_no_city_names_the_key() {
    let running = start(
        WidgetKind::WorldClock,
        WidgetHost::Tile,
        ShellFile::default(),
    );
    let text = running
        .harness
        .text_of("#widget-world-clock .sill-widget-quiet")
        .unwrap_or_default();
    assert!(text.contains("widgets.world_clocks"), "{text}");
}

#[test]
fn the_center_draws_widgets_center_in_order_under_the_history() {
    let running = center_support::opened(center_support::two_apps());
    let h = &running.harness;
    assert_eq!(h.count(".sill-nc-widgets > .ds-widget"), 3);
    let ids: Vec<_> = (1..=3)
        .map(|n| {
            h.attr(
                &format!(".sill-nc-widgets > .ds-widget:nth-child({n})"),
                "id",
            )
        })
        .collect();
    assert_eq!(
        ids,
        vec![
            Some("widget-calendar".to_owned()),
            Some("widget-up-next".to_owned()),
            Some("widget-battery".to_owned()),
        ]
    );
    let list = h.rect(".sill-nc-list").expect("the history");
    let first = h.rect("#widget-calendar").expect("the first widget");
    assert!(
        first.origin.y.0 >= list.origin.y.0 + list.size.height.0 - 0.5,
        "under the history: {list:?} {first:?}"
    );
    // A medium tile (344) fits the center's 384 px column after the panel's padding.
    let column = h.rect(".sill-nc-column").expect("the column");
    let tile = h.rect("#widget-battery").expect("a medium tile");
    assert_eq!(tile.size.width.0.round(), 344.0);
    assert!(
        tile.origin.x.0 >= column.origin.x.0 - 0.5
            && tile.origin.x.0 + tile.size.width.0 <= column.origin.x.0 + column.size.width.0 + 0.5,
        "inside the column: {column:?} {tile:?}"
    );
}

#[test]
fn on_the_desktop_the_world_clock_draws_dials_tinted_for_day_or_night() {
    let mut file = ShellFile::default();
    file.widgets.world_clocks = "Europe/London; Asia/Tokyo".to_owned();
    let running = start(WidgetKind::WorldClock, WidgetHost::Desktop, file);
    let h = &running.harness;
    assert_eq!(
        h.attr("#widget-world-clock", "data-size").as_deref(),
        Some("medium")
    );
    assert_eq!(
        h.count("#widget-world-clock .ds-clock[*|data-look=analog]"),
        2
    );
    assert_eq!(h.count("#widget-world-clock .ds-clock-dial"), 2);
    // Nine hours apart: at most one of the two can be between 06:00 and 18:00 while the other is
    // not, and each face carries a phase.
    assert_eq!(h.count("#widget-world-clock .ds-clock[*|data-phase]"), 2);
}

#[test]
fn a_battery_level_that_changes_bumps_once() {
    let mut running = start(WidgetKind::Battery, WidgetHost::Tile, ShellFile::default());
    let bumping = "#widget-battery .ds-bumped.a-bump, #widget-battery .ds-battery.a-bump";
    assert_eq!(running.harness.count(bumping), 0, "nothing bumps on mount");
    let mut battery = running.feeds.battery.borrow().clone();
    battery.level = sill_services::Percent(65);
    running.feeds.battery.send_replace(battery);
    running.harness.advance(Duration::from_millis(30));
    assert!(
        running.harness.count(bumping) >= 1,
        "the new level bumps: {}",
        running.harness.html()
    );
    running.harness.advance(Duration::from_millis(1500));
    assert_eq!(running.harness.count(bumping), 0, "and comes to rest");
}
