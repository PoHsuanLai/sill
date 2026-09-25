//! The control center's SSR render reads the frozen channels: its modules in
//! `control_center.modules` order on the two-column grid, each tile's state and status line from
//! the fake services, the levels on quire sliders, and a fed value replacing the default.

mod cc_support;

use cc_support::{count, fake, render};
use sill_services::{Bluetooth, Radio, Wifi};
use sill_settings::{ControlCenterModule, ControlCenterSettings, ShellFile};

#[test]
fn the_grid_shows_what_the_services_hold() {
    let (services, _feeds) = fake::services();
    let html = render(services, ShellFile::default());
    let cases = [
        ("seven grid rows", "class=\"sill-cc-row\"", 7),
        ("three half tiles", "data-span=\"half\"", 3),
        ("five full modules", "data-span=\"full\"", 5),
        ("a quire switch per tile", "class=\"ds-toggle\"", 3),
        ("Wi-Fi, Bluetooth and Focus on", "data-state=\"on\"", 3),
        (
            "chevrons on Wi-Fi and Bluetooth",
            "aria-label=\"Wi-Fi details\"",
            1,
        ),
        ("", "aria-label=\"Bluetooth details\"", 1),
        ("the joined network", ">home<", 1),
        ("the connected headphones", ">Buds<", 1),
        ("Do Not Disturb", ">Focus<", 1),
        ("two quire sliders", "class=\"ds-slider\"", 2),
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
    assert_eq!(count(&html, "class=\"sill-cc-row\""), 2, "one row each");
    assert_eq!(count(&html, "class=\"ds-appearance\""), 0);
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
        ("only Focus on", "data-state=\"on\"", 1),
        ("Wi-Fi off", ">Off<", 1),
        ("no adapter", ">Unavailable<", 1),
        (
            "its switch disabled",
            "aria-label=\"Bluetooth\" aria-disabled=\"true\"",
            1,
        ),
    ];
    for (name, needle, want) in cases {
        assert_eq!(count(&html, needle), want, "{name}: {needle}");
    }
}
