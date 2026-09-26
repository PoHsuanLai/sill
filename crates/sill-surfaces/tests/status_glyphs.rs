//! D1's sill lane on a real Blitz document: the fake services' values, mapped by
//! `glyph_state`, drive quire's status glyphs (quire CONSUMING "Status glyphs"), and each change
//! ends at 0 frames (design/26 R3). The bar and the control center cannot hold these glyphs yet
//! (quire's `IconButton`, `ModuleTile`, `ModulePanel` and `LevelControl` take an `Icon`: FINDINGS
//! Q390-Q392), so the specimen here is the mapping plus the glyph, fed exactly as the bar is.

mod center_support;

use center_support::fake;
use std::cell::RefCell;
use std::time::Duration;

use dioxus::prelude::*;
use ds::{
    Appearance, BatteryGlyph, BluetoothGlyph, Ds, Material, RootExtent, VolumeGlyph, WifiGlyph,
};
use ds_native::harness::{assert_settles_to_zero_frames, settle_until};
use ds_native::{Harness, Viewport};
use sill_services::{
    Attempt, Attempts, Battery, BtConnection, Charge, Connectivity, Link, Mute, Network, OpStamp,
    Percent, Presence, Radio, Services, Volume,
};
use sill_surfaces::hooks::{use_battery, use_bluetooth, use_network, use_volume, use_wifi};
use sill_surfaces::surfaces::bar::glyph_state::{
    NetworkGlyph, battery_glyph, bluetooth_glyph_state, network_glyph, volume_glyph,
};

thread_local! {
    static SERVICES: RefCell<Option<Services>> = const { RefCell::new(None) };
}

/// The four glyphs, each from its service through sill's mapping.
fn app() -> Element {
    let services = SERVICES.with(|s| s.borrow().clone());
    let Some(services) = services else {
        return rsx! {};
    };
    use_context_provider(|| services);
    rsx! {
        Ds { appearance: Appearance::default(), material: Material::Bar, extent: RootExtent::Viewport,
            Specimens {}
        }
    }
}

#[component]
fn Specimens() -> Element {
    let network = use_network()();
    let radio = use_wifi()().radio;
    let battery = battery_glyph(&use_battery()(), sill_settings::Percent(20));
    let volume = volume_glyph(&use_volume()());
    let bluetooth = bluetooth_glyph_state(&use_bluetooth()());
    rsx! {
        div { id: "wifi",
            match network_glyph(&network, radio) {
                NetworkGlyph::Wifi(state) => rsx! { WifiGlyph { state } },
                NetworkGlyph::Plain(_) => rsx! {},
            }
        }
        div { id: "battery",
            if let Some(state) = battery {
                BatteryGlyph { state }
            }
        }
        div { id: "volume", VolumeGlyph { state: volume } }
        div { id: "bluetooth", BluetoothGlyph { state: bluetooth } }
    }
}

const VIEW: Viewport = Viewport {
    width: 240,
    height: 80,
    scale_percent: 100,
};

fn start() -> (Harness, fake::Feeds) {
    let (services, feeds) = fake::services();
    SERVICES.with(|s| *s.borrow_mut() = Some(services));
    let mut harness = Harness::new(app, VIEW);
    harness.advance(Duration::from_millis(50));
    (harness, feeds)
}

fn show(harness: &Harness, glyph: &str, part: &str) -> Option<String> {
    harness.attr(&format!("#{glyph} [*|data-part={part}]"), "data-show")
}

fn lit_arcs(harness: &Harness) -> usize {
    ["dot", "arc-1", "arc-2", "arc-3"]
        .iter()
        .filter(|part| show(harness, "wifi", part).as_deref() == Some("lit"))
        .count()
}

fn network(link: Link, connectivity: Connectivity, now: Attempt) -> Network {
    Network {
        link,
        connectivity,
        join: Attempts {
            now,
            minted: OpStamp(1),
        },
    }
}

#[test]
fn a_join_searches_then_fills_and_no_internet_grows_the_badge() {
    let (mut harness, feeds) = start();
    assert_eq!(lit_arcs(&harness), 4, "the fake joins at 72 %: three arcs");
    assert_settles_to_zero_frames(&mut harness);
    // Connecting is Joining with the service's stamp, never "Wi-Fi off" (the bug).
    feeds.network.send_replace(network(
        Link::Connecting,
        Connectivity::Unknown,
        Attempt::Running(OpStamp(1)),
    ));
    settle_until(&mut harness, |h| lit_arcs(h) == 1);
    assert_eq!(
        harness.count("#wifi [*|data-part=slash]"),
        0,
        "joining is not off"
    );
    let joined = Link::Wifi {
        ssid: "home".to_owned(),
        strength: Percent(50),
    };
    feeds
        .network
        .send_replace(network(joined.clone(), Connectivity::Full, Attempt::Quiet));
    settle_until(&mut harness, |h| lit_arcs(h) == 3);
    assert_settles_to_zero_frames(&mut harness);
    feeds
        .network
        .send_replace(network(joined, Connectivity::Portal, Attempt::Quiet));
    settle_until(&mut harness, |h| {
        show(h, "wifi", "badge").as_deref() == Some("lit")
    });
    assert_settles_to_zero_frames(&mut harness);
}

#[test]
fn a_failed_join_shakes_once_and_the_radio_off_slashes() {
    let (mut harness, feeds) = start();
    let failed = network(
        Link::Disconnected,
        Connectivity::Unknown,
        Attempt::Failed(OpStamp(2)),
    );
    feeds.network.send_replace(failed.clone());
    settle_until(&mut harness, |h| {
        h.has_class("#wifi .ds-status-glyph", "a-shake-x")
    });
    assert_settles_to_zero_frames(&mut harness);
    feeds.network.send_replace(failed);
    harness.advance(Duration::from_millis(60));
    assert!(
        !harness.has_class("#wifi .ds-status-glyph", "a-shake-x"),
        "the same stamp shook again"
    );
    feeds.wifi.send_modify(|wifi| wifi.radio = Radio::Off);
    settle_until(&mut harness, |h| h.count("#wifi [*|data-part=slash]") == 1);
    assert_settles_to_zero_frames(&mut harness);
}

#[test]
fn the_battery_fill_turns_low_at_the_key_and_charging_shows_the_bolt() {
    let (mut harness, feeds) = start();
    let at = |level, charge| Battery {
        presence: Presence::Present,
        level: Percent(level),
        charge,
        ..Battery::default()
    };
    assert_settles_to_zero_frames(&mut harness);
    feeds.battery.send_replace(at(18, Charge::Discharging));
    settle_until(&mut harness, |h| {
        h.attr("#battery [*|data-part=fill]", "data-tone")
            .as_deref()
            == Some("low")
    });
    assert_settles_to_zero_frames(&mut harness);
    feeds.battery.send_replace(at(18, Charge::Charging));
    settle_until(&mut harness, |h| {
        show(h, "battery", "bolt").as_deref() == Some("lit")
    });
    assert_eq!(
        harness
            .attr("#battery [*|data-part=fill]", "data-tone")
            .as_deref(),
        Some("normal"),
        "charging is never low"
    );
    assert_settles_to_zero_frames(&mut harness);
    feeds.battery.send_replace(at(100, Charge::Full));
    settle_until(&mut harness, |h| {
        show(h, "battery", "plug").as_deref() == Some("lit")
    });
    assert_settles_to_zero_frames(&mut harness);
    feeds.battery.send_replace(Battery::default());
    settle_until(&mut harness, |h| h.count("#battery .ds-status-glyph") == 0);
    assert_settles_to_zero_frames(&mut harness);
}

#[test]
fn volume_zero_keeps_the_slash_and_the_waves_follow() {
    let (mut harness, feeds) = start();
    let on = |h: &Harness, part: &str| {
        h.attr(&format!("#volume [*|data-part={part}]"), "data-on")
            .as_deref()
            == Some("on")
    };
    assert!(
        on(&harness, "wave-2") && !on(&harness, "wave-3"),
        "40 %: two waves"
    );
    assert_settles_to_zero_frames(&mut harness);
    feeds.volume.send_modify(|v| v.level = Percent(0));
    settle_until(&mut harness, |h| on(h, "slash"));
    assert!(!on(&harness, "wave-1"), "no waves at zero");
    assert_settles_to_zero_frames(&mut harness);
    feeds.volume.send_replace(Volume {
        presence: Presence::Present,
        level: Percent(90),
        mute: Mute::Audible,
        sink: "Speakers".to_owned(),
    });
    settle_until(&mut harness, |h| on(h, "wave-3") && !on(h, "slash"));
    assert_settles_to_zero_frames(&mut harness);
}

#[test]
fn bluetooth_breathes_while_connecting_and_seals_when_connected() {
    let (mut harness, feeds) = start();
    assert_settles_to_zero_frames(&mut harness);
    feeds.bluetooth.send_modify(|bt| {
        bt.devices[0].connection = BtConnection::Connecting;
        bt.connect = Attempts {
            now: Attempt::Running(OpStamp(1)),
            minted: OpStamp(1),
        };
    });
    settle_until(&mut harness, |h| {
        matches!(
            h.attr("#bluetooth .ds-status-glyph", "data-pending")
                .as_deref(),
            Some("high" | "low")
        )
    });
    feeds.bluetooth.send_modify(|bt| {
        bt.devices[0].connection = BtConnection::Connected;
        bt.connect.now = Attempt::Quiet;
    });
    settle_until(&mut harness, |h| {
        show(h, "bluetooth", "dots").as_deref() == Some("lit")
    });
    assert_settles_to_zero_frames(&mut harness);
}
