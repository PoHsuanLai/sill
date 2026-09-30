//! The bar mounted as a `Surface` on the fake services: what the services hold is what the bar
//! shows, and a value fed after it started changes what it shows.

use ds_harness::Driver;
use std::time::Duration;

use sill_bar::Bar;
use sill_model::battery::Battery;
use sill_model::clock::ClockText;
use sill_model::cosmic_wl::Toplevels;
use sill_model::units::Presence;
use sill_ui_kit::harness::SurfaceHarness;
use sill_ui_kit::links::ControlCenterLink;

/// The bar on the default settings, its first frame drawn.
fn start() -> SurfaceHarness<Bar> {
    let mut bar = SurfaceHarness::<Bar>::setup(ControlCenterLink::new(), ()).start();
    bar.harness.advance(Duration::ZERO);
    bar
}

/// How many times `needle` occurs in `html`.
fn count(html: &str, needle: &str) -> usize {
    html.matches(needle).count()
}

#[test]
fn the_bar_shows_what_the_services_hold() {
    let html = start().harness.html();
    let cases = [
        ("the clock", "Thu 24 Sep 14:05", 1),
        ("the app name", ">foot<", 1),
        ("the volume item", "aria-label=\"Volume\"", 1),
        ("the network item", "aria-label=\"Network\"", 1),
        ("the battery item", "aria-label=\"Battery\"", 1),
        ("in one segmented group", "class=\"ds-ws-pills\"", 1),
        ("each a quire pill", "class=\"ds-ws-pill\"", 3),
        ("one current pill", "aria-current=\"true\"", 1),
        (
            "the app name and the clock as menu-bar title items",
            "data-image=\"leading\"",
            2,
        ),
        ("the app name bold", "data-emphasis=\"strong\"", 1),
        ("the shell type scale", "--shell-bar-font:13px;", 1),
        (
            "22 px menu rows for the popups",
            "--shell-menu-row-h:22px;",
            1,
        ),
        ("the material stack", "--m-vibrancy:1;", 1),
        ("the Bar material", "data-material=\"bar\"", 1),
        ("the root draws the Space", "data-frame=\"tinted\"", 1),
        ("on the frame ground", "data-ground=\"frame\"", 1),
        ("no tint layer of the bar's own", "sill-bar-tint", 0),
        // Sound, Wi-Fi and Battery (`control_center.menu_bar_*`), then the control center.
        ("four status items", "data-image=\"only\"", 4),
        (
            "the control center item",
            "aria-label=\"Control Center\"",
            1,
        ),
        (
            "their metrics",
            "--bar-status-box:22px;--bar-status-glyph:16px;",
            1,
        ),
        ("no menu open", "aria-expanded=\"true\"", 0),
    ];
    for (name, needle, want) in cases {
        assert_eq!(count(&html, needle), want, "{name}: {needle}\n{html}");
    }
}

#[test]
fn fed_values_replace_the_defaults() {
    let mut bar = start();
    let before = bar.harness.html();
    let feeds = &bar.feeds;
    feeds.clock.send_replace(ClockText {
        text: "Fri 25 Sep 09:00".to_owned(),
    });
    feeds.battery.send_replace(Battery {
        presence: Presence::Absent,
        ..Battery::default()
    });
    feeds
        .cosmic_wl
        .send_modify(|wl| wl.toplevels = Toplevels::default());
    bar.harness.advance(Duration::from_millis(40));
    let after = bar.harness.html();
    let cases = [
        ("the old time", "Thu 24 Sep 14:05", 1, 0),
        ("the new time", "Fri 25 Sep 09:00", 0, 1),
        ("the battery hides", "aria-label=\"Battery\"", 1, 0),
        ("no window: the desktop", ">Desktop<", 0, 1),
    ];
    for (name, needle, was, now) in cases {
        assert_eq!(
            (count(&before, needle), count(&after, needle)),
            (was, now),
            "{name}"
        );
    }
}
