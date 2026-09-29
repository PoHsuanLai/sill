//! The control center's SSR render reads the frozen channels: its modules in
//! `control_center.modules` order on quire's `ModuleGrid` (columns and gap from the settings),
//! each tile's state and status line from the fake services, the full-width modules on quire's
//! `ModulePanel` (bare in a module's dropdown), the levels on quire's `LevelControl`, and a fed
//! value replacing the default.

mod cc_support;

use std::time::Duration;

use cc_support::live::{self, Report};
use cc_support::{count, fake, render, render_dropdown};
use ds::ShortcutKey;
use ds_settings::{Count, Px};
use sill_model::bluetooth::Bluetooth;
use sill_model::wifi::{Radio, Ssid, Wifi, WifiCommand};
use sill_settings::{ControlCenterModule, ControlCenterSettings, ShellFile};
use sill_ui_kit::surfaces::control_center::{CcPage, Detail};

#[test]
fn the_grid_shows_what_the_services_hold() {
    let (services, _feeds) = fake::services();
    let html = render(services, ShellFile::default());
    let cases = [
        ("quire's module grid", "class=\"ds-module-grid\"", 1),
        ("three quire tiles", "class=\"ds-module-tile\"", 3),
        ("each a column", "data-span=\"half\"", 3),
        (
            "five full-width quire panels",
            "class=\"ds-module-panel\"",
            5,
        ),
        ("each on the tile's plate", "data-plate=\"tile\"", 5),
        (
            "the grid from control_center.grid_*, the panel padding it",
            "style=\"--grid-columns:2;--grid-gap:10px;--grid-padding:0px\"",
            1,
        ),
        (
            "Sound, Display and Battery headed",
            "class=\"ds-module-panel-head\"",
            3,
        ),
        ("the compact picker", "data-layout=\"compact\"", 1),
        ("no plate of sill's", "sill-cc-module", 0),
        (
            "Wi-Fi, Bluetooth and Focus on",
            "data-state=\"on\" aria-pressed=\"true\"",
            3,
        ),
        (
            "chevrons on Wi-Fi and Bluetooth",
            "aria-label=\"Wi-Fi details\"",
            1,
        ),
        ("", "aria-label=\"Bluetooth details\"", 1),
        ("the joined network", ">home<", 1),
        ("no device named on the Bluetooth tile", ">Buds<", 0),
        ("Bluetooth and Do Not Disturb on", ">On<", 2),
        ("Do Not Disturb", ">Focus<", 1),
        ("two quire level controls", "class=\"ds-level\"", 2),
        (
            "the sink as Sound's title",
            ">Built-in Audio Analog Stereo<",
            1,
        ),
        ("its level", ">40%<", 1),
        ("Display", ">Display<", 1),
        ("nothing playing", ">Not Playing<", 1),
        ("quire's appearance picker", "class=\"ds-appearance\"", 1),
        ("the battery's time", "2:05 remaining", 1),
        ("the power modes", ">Performance<", 1),
        ("no raw button of sill's", "<button class=\"sill", 0),
    ];
    for (name, needle, want) in cases {
        assert_eq!(count(&html, needle), want, "{name}: {needle}");
    }
}

#[test]
fn modules_follow_the_settings_order() {
    let (services, _feeds) = fake::services();
    let file = ShellFile {
        control_center: ControlCenterSettings {
            modules: vec![ControlCenterModule::Sound, ControlCenterModule::Wifi],
            ..ControlCenterSettings::default()
        },
        ..ShellFile::default()
    };
    let html = render(services, file);
    let sound = html.find("Built-in Audio").expect("the Sound module");
    let wifi = html.find(">Wi-Fi<").expect("the Wi-Fi tile");
    assert!(sound < wifi, "Sound comes first");
    assert_eq!(
        count(&html, "class=\"ds-module-tile\""),
        1,
        "the Wi-Fi tile"
    );
    assert_eq!(
        count(&html, "class=\"ds-module-panel\""),
        1,
        "the Sound module"
    );
    assert_eq!(count(&html, "class=\"ds-appearance\""), 0);
}

#[test]
fn the_grid_takes_its_columns_and_gap_from_the_settings() {
    let (services, _feeds) = fake::services();
    let file = ShellFile {
        control_center: ControlCenterSettings {
            grid_columns: Count(3),
            grid_gap_px: Px(12),
            ..ControlCenterSettings::default()
        },
        ..ShellFile::default()
    };
    let html = render(services, file);
    assert_eq!(
        count(
            &html,
            "style=\"--grid-columns:3;--grid-gap:12px;--grid-padding:0px\""
        ),
        1
    );
}

#[test]
fn a_modules_dropdown_draws_it_bare_on_the_card() {
    let (services, _feeds) = fake::services();
    let html = render_dropdown(services, ShellFile::default(), ControlCenterModule::Sound);
    let cases = [
        ("one quire panel", "class=\"ds-module-panel\"", 1),
        ("bare: the card is its plate", "data-plate=\"bare\"", 1),
        ("headed", "class=\"ds-module-panel-head\"", 1),
        ("its level", ">40%<", 1),
    ];
    for (name, needle, want) in cases {
        assert_eq!(count(&html, needle), want, "{name}: {needle}");
    }
}

#[test]
fn fed_values_replace_the_defaults() {
    let (services, feeds) = fake::services();
    feeds.wifi.send_replace(Wifi {
        radio: Radio::Off,
        ..fake::wifi()
    });
    feeds.bluetooth.send_replace(Bluetooth::default());
    let html = render(services, ShellFile::default());
    let cases = [
        (
            "only Focus on",
            "data-state=\"on\" aria-pressed=\"true\"",
            1,
        ),
        ("Wi-Fi off", ">Off<", 1),
        ("no adapter", ">Unavailable<", 1),
        (
            "its tile disabled",
            "aria-pressed=\"false\" aria-disabled=\"true\"",
            1,
        ),
    ];
    for (name, needle, want) in cases {
        assert_eq!(count(&html, needle), want, "{name}: {needle}");
    }
}

#[test]
fn a_chevron_slides_its_pane_in_and_escape_unwinds() {
    let (services, feeds) = fake::services();
    feeds.wifi.send_replace(Wifi {
        radio: Radio::Off,
        ..fake::wifi()
    });
    let mut running = live::start(services, feeds);
    let step = Duration::from_millis(50);
    // The popover places its card a frame after it measures the anchor.
    running.harness.advance(Duration::from_millis(300));
    let chevron = running
        .harness
        .centre(".ds-module-chevron")
        .expect("the Wi-Fi chevron is laid out");
    // A press in the panel reaches it (the popup frame is in flow, so the hit test descends):
    // the Wi-Fi tile asks for the radio (a fake service: nothing real changes).
    let switch = running
        .harness
        .centre(".ds-module-title")
        .expect("the Wi-Fi tile");
    running.harness.click(switch);
    running.harness.advance(step);
    assert_eq!(
        running.feeds.wifi_commands.try_recv().ok(),
        Some(WifiCommand::Radio(Radio::On))
    );
    running.harness.click(chevron);
    // quire's PaneSwitcher slides the detail in and drops the grid once it settles.
    running.harness.advance(Duration::from_millis(600));
    assert_eq!(
        running.harness.attr(".ds-panes", "data-shown").as_deref(),
        Some("detail")
    );
    assert_eq!(
        running.harness.count(".sill-cc-detail"),
        1,
        "the Wi-Fi pane"
    );
    assert_eq!(
        running.harness.count(".ds-module-grid"),
        0,
        "the grid is gone"
    );
    assert_eq!(
        running.harness.text_of(".sill-cc-empty").as_deref(),
        Some("Wi-Fi is off")
    );
    // Escape with the pressed chevron gone: the panel holds the keyboard.
    running.harness.key(ShortcutKey::Escape);
    running.harness.advance(Duration::from_millis(600));
    assert_eq!(
        running.harness.attr(".ds-panes", "data-shown").as_deref(),
        Some("root")
    );
    assert_eq!(
        running.harness.count(".ds-module-grid"),
        1,
        "back on the grid"
    );
    assert_eq!(
        running.reports(),
        [
            Report::Page(CcPage::Detail(Detail::Wifi)),
            Report::Page(CcPage::Modules)
        ]
    );
    // Escape on the grid reaches the popover, which closes.
    running.harness.key(ShortcutKey::Escape);
    running.harness.advance(Duration::from_millis(400));
    assert_eq!(running.reports().last(), Some(&Report::Closed));
}

#[test]
fn now_playing_shows_the_track_and_its_transport() {
    let (services, feeds) = fake::services();
    feeds.media.send_replace(fake::media());
    let html = render(services, ShellFile::default());
    let cases = [
        ("the title", ">So What<", 1),
        ("the artist and album", ">Miles Davis — Kind of Blue<", 1),
        ("previous", "aria-label=\"Previous\"", 1),
        ("pause while playing", "aria-label=\"Pause\"", 1),
        ("next", "aria-label=\"Next\"", 1),
        ("no empty module", ">Not Playing<", 0),
    ];
    for (name, needle, want) in cases {
        assert_eq!(count(&html, needle), want, "{name}: {needle}");
    }
}

#[test]
fn the_wifi_pane_lists_networks_and_a_row_joins_one() {
    let (services, feeds) = fake::services();
    let mut running = live::start(services, feeds);
    running.harness.advance(Duration::from_millis(300));
    let chevron = running
        .harness
        .centre(".ds-module-chevron")
        .expect("the Wi-Fi chevron");
    running.harness.click(chevron);
    running.harness.advance(Duration::from_millis(600));
    // The pane asks for a scan as it opens (a fake service: nothing real changes).
    assert_eq!(
        running.feeds.wifi_commands.try_recv().ok(),
        Some(WifiCommand::Scan)
    );
    // home (joined, checked), office (saved, locked), cafe (new, open): quire SettingsRows.
    assert_eq!(running.harness.count(".ds-settings-row"), 3);
    assert_eq!(
        running.harness.text_of(".ds-settings-row-title").as_deref(),
        Some("home")
    );
    let office = running
        .harness
        .centre(".ds-settings-row:nth-child(2)")
        .expect("the office row");
    running.harness.click(office);
    running.harness.advance(Duration::from_millis(50));
    assert_eq!(
        running.feeds.wifi_commands.try_recv().ok(),
        Some(WifiCommand::Connect(Ssid("office".to_owned())))
    );
}
