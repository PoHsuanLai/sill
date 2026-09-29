//! The bar's SSR render reads the frozen channels: what the fake services hold is what the bar
//! shows, and a value fed before the render changes what it shows.

mod support;

use dioxus::prelude::*;
use sill_model::battery::Battery;
use sill_model::clock::ClockText;
use sill_model::cosmic_wl::Toplevels;
use sill_model::units::Presence;
use sill_settings::{SettingsFeed, ShellFile};
use sill_surfaces::Bar;
use sill_surfaces::services::Services;
use sill_surfaces::surfaces::bar::BarProps;

fn render(services: Services) -> String {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .expect("a runtime builds"); // test-only
    let _entered = rt.enter();
    let mut dom = VirtualDom::new_with_props(
        Bar,
        BarProps {
            output: support::fake::output(),
        },
    );
    dom.provide_root_context(services);
    dom.provide_root_context(SettingsFeed::fixed(ShellFile::default()));
    dom.rebuild_in_place();
    dioxus_ssr::render(&dom)
}

/// How many times `needle` occurs in `html`.
fn count(html: &str, needle: &str) -> usize {
    html.matches(needle).count()
}

#[test]
fn the_bar_shows_what_the_services_hold() {
    let (services, _feeds) = support::fake::services();
    let html = render(services);
    let cases = [
        ("the clock", "Thu 24 Sep 14:05", 1),
        ("the app name", ">foot<", 1),
        ("the volume item", "aria-label=\"Volume\"", 1),
        ("the network item", "aria-label=\"Network\"", 1),
        ("the battery item", "aria-label=\"Battery\"", 1),
        ("three workspace pills", "class=\"sill-bar-space\"", 3),
        ("in one segmented group", "class=\"ds-ws-pills\"", 1),
        ("each a quire pill", "class=\"ds-ws-pill\"", 3),
        ("one current pill", "aria-current=\"true\"", 1),
        (
            "the app name and the clock as menu-bar items",
            "class=\"ds-bar-item\"",
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
        ("four status items", "data-variant=\"status\"", 4),
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
    let (services, feeds) = support::fake::services();
    let before = render(services.clone());
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
    let after = render(services);
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
