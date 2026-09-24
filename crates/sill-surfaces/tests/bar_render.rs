//! The bar's SSR render reads the frozen channels: what the fake services hold is what the bar
//! shows, and a value fed before the render changes what it shows.

mod support;

use dioxus::prelude::*;
use sill_services::{Battery, ClockText, Presence, Services, Toplevels};
use sill_settings::{SettingsFeed, ShellFile};
use sill_surfaces::Bar;
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
        ("one active pill", "aria-pressed=\"true\"", 1),
        ("the Bar material", "data-material=\"bar\"", 1),
        ("no menu open", "data-open=\"true\"", 0),
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
    feeds.toplevels.send_replace(Toplevels::default());
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
