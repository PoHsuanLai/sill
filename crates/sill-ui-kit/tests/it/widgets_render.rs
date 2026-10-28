//! What the notification center's widgets draw, each in a `ds_harness::Harness` on the fake
//! services: the card (quire's `WidgetCard`, a tile at Medium, `data-widget` its kind) and Up
//! Next's rows, never one past the tile's body.

use sill_ui_kit::harness::fake;
use std::cell::RefCell;
use std::time::Duration;

use dioxus::prelude::*;
use ds::prelude::*;
use ds_harness::{Clock, Harness, HarnessConfig, Viewport};
use sill_settings::ShellFile;
use sill_ui_kit::hooks::PinnedAppearance;
use sill_ui_kit::services::Services;
use sill_ui_kit::settings::SettingsFeed;
use sill_ui_kit::widgets::{CenterWidget, Widget, widgets_css};

/// What the harness's root provides.
#[derive(Clone)]
struct World {
    services: Services,
    settings: SettingsFeed,
    kind: CenterWidget,
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
    use_context_provider(PinnedAppearance::default);
    use_context_provider(|| world.settings.clone());
    rsx! {
        Ds { sheet: Some(sill_shell_kit::stylesheet()), appearance: Appearance::default(), material: Material::Popover, extent: RootExtent::Viewport,
            AppStyle { css: widgets_css() }
            div { style: "width:344px;",
                Widget { kind: world.kind }
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

fn start(kind: CenterWidget, file: ShellFile) -> Running {
    let (services, feeds) = fake::services();
    WORLD.with(|w| {
        *w.borrow_mut() = Some(World {
            services,
            settings: SettingsFeed::fixed(file),
            kind,
        });
    });
    let mut harness = Harness::new(app, HarnessConfig::new(VIEWPORT).with_clock(Clock::Virtual));
    harness.advance(Duration::from_millis(100));
    Running { harness, feeds }
}

#[test]
fn in_the_center_the_frame_is_quires_tile_with_no_header_row() {
    let running = start(CenterWidget::Calendar, ShellFile::default());
    let h = &running.harness;
    assert_eq!(h.count(".ds-widget#widget-calendar"), 1);
    assert_eq!(
        h.attr("#widget-calendar", "data-size").as_deref(),
        Some("medium")
    );
    assert_eq!(
        h.count("#widget-calendar .ds-widget-title"),
        0,
        "the month has no header row (design/23-WIDGETS.md M25)"
    );
    assert_eq!(
        h.count(".ds[*|data-material=widget]"),
        0,
        "no second material"
    );
    let tile = h.rect("#widget-calendar").expect("the tile is laid out");
    assert_eq!(
        (tile.size.width.0.round(), tile.size.height.0.round()),
        (344.0, 164.0)
    );
}

#[test]
fn the_card_names_its_kind() {
    let cases = [
        (CenterWidget::Calendar, "quire.month"),
        (CenterWidget::UpNext, "sill.up-next"),
    ];
    for (kind, name) in cases {
        let running = start(kind, ShellFile::default());
        let h = &running.harness;
        assert_eq!(h.attr(".ds-widget", "data-widget").as_deref(), Some(name));
        assert_eq!(
            h.attr(".ds-widget", "data-tint"),
            None,
            "a tile lays no second tint on the center's Popover"
        );
    }
}

/// The most events asked for (3) asks for more events than a Medium tile's body has room for at the
/// default 164 px cell. With more qualifying events than fit, no row's rect may reach past the
/// card's own body rect (two rows at Medium).
#[test]
fn up_next_never_clips_a_row_past_the_tiles_body() {
    let file = ShellFile::default();
    let want = 2;
    {
        let mut running = start(CenterWidget::UpNext, file.clone());
        let at = jiff::Timestamp::now();
        let hour = |h: i64| at.checked_add(jiff::ToSpan::hours(h)).unwrap_or(at);
        let event = |uid: &str, hours_ahead: i64| sill_model::events::CalEvent {
            uid: sill_model::events::EventUid(uid.to_owned()),
            title: uid.to_owned(),
            when: sill_model::events::EventTime::Timed {
                start: hour(hours_ahead),
                end: hour(hours_ahead),
            },
            location: None,
            calendar: sill_model::events::CalendarName("home".to_owned()),
        };
        let events = vec![
            event("one", 1),
            event("two", 2),
            event("three", 3),
            event("four", 4),
            event("five", 5),
            event("six", 6),
            event("seven", 7),
        ];
        running
            .feeds
            .agenda
            .send_replace(sill_model::events::Agenda {
                today: at.to_zoned(jiff::tz::TimeZone::UTC).date(),
                events,
                status: sill_model::events::SourceStatus::Read {
                    calendars: 1,
                    problems: Vec::new(),
                },
            });
        running.harness.advance(Duration::from_millis(60));
        let h = &running.harness;
        let body = h
            .rect("#widget-up-next .ds-widget-body")
            .expect("the frame's body is laid out");
        let bottom = body.origin.y.0 + body.size.height.0;
        let row_count = h.count("#widget-up-next .sill-un-row");
        assert_eq!(row_count, want, "the tile holds {want} whole rows");
        for n in 1..=row_count {
            let row = h
                .rect(&format!("#widget-up-next .sill-un-row:nth-child({n})"))
                .unwrap_or_else(|| panic!("row {n} is laid out"));
            assert!(
                row.origin.y.0 + row.size.height.0 <= bottom + 0.5,
                "row {n} clips the tile's body: {row:?} body_bottom={bottom}"
            );
        }
    }
}
