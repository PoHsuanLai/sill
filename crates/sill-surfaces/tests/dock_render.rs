//! The dock's SSR render reads the frozen channels: pinned apps then running ones grouped by
//! app, the Trash, running dots, the badge, the ring, the regions' elements; a value fed before
//! the render changes what it shows.

mod dock_support;

use dioxus::prelude::*;
use sill_launcher::AppId;
use sill_services::{Badges, DockPins, Services, Toplevels};
use sill_settings::{SettingsFeed, ShellFile, TrashTile};
use sill_surfaces::Dock;
use sill_surfaces::surfaces::dock::DockProps;

fn render_with(services: Services, file: ShellFile) -> String {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .expect("a runtime builds"); // test-only
    let _entered = rt.enter();
    let mut dom = VirtualDom::new_with_props(
        Dock,
        DockProps {
            output: dock_support::fake::output(),
        },
    );
    dom.provide_root_context(services);
    dom.provide_root_context(SettingsFeed::fixed(file));
    dom.rebuild_in_place();
    dioxus_ssr::render(&dom)
}

fn render(services: Services) -> String {
    render_with(services, ShellFile::default())
}

/// How many times `needle` occurs in `html`.
fn count(html: &str, needle: &str) -> usize {
    html.matches(needle).count()
}

/// The hover labels in document order: each tile's name.
fn labels(html: &str) -> Vec<String> {
    html.split("class=\"ds-fly\" role=\"tooltip\">")
        .skip(1)
        .filter_map(|rest| rest.split('<').next().map(str::to_owned))
        .collect()
}

#[test]
fn pinned_then_running_then_trash_with_dots_and_a_badge() {
    let (services, _feeds) = dock_support::services();
    let html = render(services);
    assert_eq!(
        labels(&html),
        ["COSMIC Files", "COSMIC Terminal", "Firefox", "Trash"],
        "{html}"
    );
    let cases = [
        ("four tiles", "class=\"sill-dock-tile\"", 4),
        ("terminal and firefox run", "class=\"sill-dock-dot\"", 2),
        ("firefox's count", ">7</span>", 1),
        ("one badge", "class=\"sill-dock-badge\"", 1),
        ("firefox's ring", "class=\"sill-dock-ring\"", 1),
        ("the hit element", "id=\"dock-hit\"", 1),
        ("the pill element", "id=\"dock-pill\"", 1),
        (
            "the separator before Trash",
            "class=\"sill-dock-separator\"",
            1,
        ),
        ("no strip while auto-hide is off", "id=\"dock-strip\"", 0),
        ("the Dock material", "data-material=\"dock\"", 1),
        ("the root draws the Space", "data-frame=\"tinted\"", 1),
        ("no tint layer of the dock's own", "sill-dock-tint", 0),
        ("plates on paper", "data-ground=\"frame\"", 0),
        (
            "Files and Terminal as themed images",
            "data-kind=\"image\"",
            2,
        ),
    ];
    for (name, needle, want) in cases {
        let got = count(&html, needle);
        // Icons need this machine's hicolor theme; the rest does not.
        if name.contains("themed") && got == 0 {
            continue;
        }
        assert_eq!(got, want, "{name}: {needle}\n{html}");
    }
}

#[test]
fn fed_values_replace_the_defaults() {
    let (services, feeds) = dock_support::services();
    let before = render(services.clone());
    feeds.pins.send_replace(DockPins {
        version: 1,
        pinned: vec![AppId(dock_support::FIREFOX.to_owned())],
    });
    feeds.bar.toplevels.send_replace(Toplevels::default());
    feeds.badges.send_replace(Badges::default());
    let after = render(services);
    assert_eq!(labels(&after), ["Firefox", "Trash"]);
    let cases = [
        ("no window, no dot", "class=\"sill-dock-dot\"", 2, 0),
        ("no badge", "class=\"sill-dock-badge\"", 1, 0),
    ];
    for (name, needle, was, now) in cases {
        assert_eq!(
            (count(&before, needle), count(&after, needle)),
            (was, now),
            "{name}"
        );
    }
}

#[test]
fn settings_drop_the_trash_and_add_the_strip() {
    let (services, _feeds) = dock_support::services();
    let mut file = ShellFile::default();
    file.dock.trash = TrashTile::Off;
    file.dock.autohide = sill_settings::AutoHide::On;
    let html = render_with(services, file);
    assert_eq!(
        labels(&html),
        ["COSMIC Files", "COSMIC Terminal", "Firefox"]
    );
    assert_eq!(count(&html, "class=\"sill-dock-separator\""), 0);
    assert_eq!(count(&html, "id=\"dock-strip\""), 1);
}
