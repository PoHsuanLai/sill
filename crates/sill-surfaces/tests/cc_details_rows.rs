//! Row details on the real control center (design/26 5.2.2, 5.2.3): the networks and devices
//! panes' row phases from the services' stamps (`Network.join`,
//! `Bluetooth.connect`), driven by the fake services on `Clock::Virtual`. Every state change ends
//! at 0 frames (R3). Nothing here reaches a real radio.

mod cc_support;

use cc_support::details::{center, figure, ms, open, press};
use cc_support::fake;

use ds_native::Harness;
use ds_native::harness::assert_settles_to_zero_frames;
use sill_model::attempt::{Attempt, Attempts, OpStamp};
use sill_model::network::{Connectivity, Link, Network};
use sill_model::units::Percent;
use sill_model::wifi::{AccessPoint, Association, Security, Ssid, Wifi};
use sill_services::{BtBattery, BtConnection};

// ---- Wi-Fi networks -------------------------------------------------------------------------

/// Wi-Fi with `home`, `office` (both saved and secured) and `cafe` at these associations.
fn wifi(home: Association, office: Association) -> Wifi {
    let ap = |ssid: &str, strength, security, association| AccessPoint {
        ssid: Ssid(ssid.to_owned()),
        strength: Percent(strength),
        security,
        association,
    };
    Wifi {
        access_points: vec![
            ap("home", 72, Security::Secured, home),
            ap("office", 48, Security::Secured, office),
            ap("cafe", 30, Security::Open, Association::Idle),
        ],
        ..fake::wifi()
    }
}

fn join(link: Link, now: Attempt, minted: u32) -> Network {
    Network {
        link,
        connectivity: Connectivity::Full,
        join: Attempts {
            now,
            minted: OpStamp(minted),
        },
    }
}

fn office_link() -> Link {
    Link::Wifi {
        ssid: "office".to_owned(),
        strength: Percent(48),
    }
}

fn row(n: usize) -> String {
    format!(".sill-cc-rows > .ds-settings-row:nth-child({n})")
}

fn title(harness: &Harness, n: usize) -> Option<String> {
    harness.text_of(&format!("{} .ds-settings-row-title", row(n)))
}

#[test]
fn a_network_joins_with_a_spinner_where_its_lock_was_then_seals_its_disc() {
    let mut running = center(|feeds| {
        feeds
            .wifi
            .send_replace(wifi(Association::Idle, Association::Idle));
        feeds
            .network
            .send_replace(join(Link::Disconnected, Attempt::Quiet, 4));
    });
    open(&mut running.harness, "Wi-Fi details");
    assert_settles_to_zero_frames(&mut running.harness);
    // office starts joining: the service's stamp 5 is the row's.
    running
        .feeds
        .wifi
        .send_replace(wifi(Association::Idle, Association::Connecting));
    running
        .feeds
        .network
        .send_replace(join(Link::Connecting, Attempt::Running(OpStamp(5)), 5));
    let harness = &mut running.harness;
    harness.advance(ms(0));
    assert_eq!(title(harness, 2).as_deref(), Some("office"));
    assert_eq!(harness.attr(&row(2), "aria-busy").as_deref(), Some("true"));
    harness.advance(ms(399));
    assert_eq!(
        harness.count(&format!("{} .ds-spinner", row(2))),
        0,
        "a fast join shows nothing"
    );
    harness.advance(ms(1));
    assert_eq!(harness.count(&format!("{} .ds-spinner", row(2))), 1);
    assert_eq!(harness.count(".ds-spinner"), 1, "only the joining row");
    harness.advance(ms(10_000));
    assert_eq!(
        harness
            .attr(&format!("{} .ds-spinner", row(2)), "data-pending")
            .as_deref(),
        Some("still"),
        "held still at the cap"
    );
    assert_settles_to_zero_frames(harness);
    // It lands: office moves to the top, keeps its row, and its disc seals once (from the
    // service, not a press: no spring).
    running
        .feeds
        .wifi
        .send_replace(wifi(Association::Idle, Association::Connected));
    running
        .feeds
        .network
        .send_replace(join(office_link(), Attempt::Quiet, 5));
    let harness = &mut running.harness;
    harness.advance(ms(0));
    assert_eq!(title(harness, 1).as_deref(), Some("office"));
    let glyph = format!("{} .ds-settings-row-glyph", row(1));
    assert!(
        harness
            .attr(&glyph, "class")
            .unwrap_or_default()
            .contains("a-seal-out"),
        "{:?}",
        harness.attr(&glyph, "class")
    );
    assert_eq!(harness.attr(&glyph, "data-disc").as_deref(), Some("on"));
    assert_eq!(
        harness.attr(&row(1), "aria-pressed"),
        None,
        "no check on the network in use: the disc says it"
    );
    assert_eq!(
        harness.count(&format!("{} .ds-settings-row-trail svg.ds-ic", row(1))),
        1,
        "the lock is back"
    );
    assert_eq!(
        harness
            .attr(&format!("{} .ds-settings-row-glyph", row(2)), "data-disc")
            .as_deref(),
        Some("off")
    );
    assert_settles_to_zero_frames(harness);
    assert!(
        !harness
            .attr(&glyph, "class")
            .unwrap_or_default()
            .contains("a-seal-out"),
        "the seal plays once"
    );
}

#[test]
fn a_failed_join_shakes_its_row_once_and_says_so() {
    let mut running = center(|feeds| {
        feeds
            .wifi
            .send_replace(wifi(Association::Connecting, Association::Idle));
        feeds
            .network
            .send_replace(join(Link::Connecting, Attempt::Running(OpStamp(6)), 6));
    });
    open(&mut running.harness, "Wi-Fi details");
    assert_eq!(title(&running.harness, 1).as_deref(), Some("home"));
    // A join already running when the pane opens is pending; past the cap it holds still.
    running.harness.advance(ms(10_000));
    assert_settles_to_zero_frames(&mut running.harness);
    running
        .feeds
        .wifi
        .send_replace(wifi(Association::Idle, Association::Idle));
    running
        .feeds
        .network
        .send_replace(join(Link::Disconnected, Attempt::Failed(OpStamp(7)), 7));
    let harness = &mut running.harness;
    harness.advance(ms(0));
    assert!(harness.has_class(&row(1), "a-shake-x"));
    assert_eq!(
        harness
            .text_of(&format!("{} .ds-settings-row-detail", row(1)))
            .as_deref(),
        Some("Couldn’t connect")
    );
    assert_settles_to_zero_frames(harness);
    // The same failure again (a re-poll) is no moment (R6).
    running
        .feeds
        .network
        .send_replace(join(Link::Disconnected, Attempt::Failed(OpStamp(7)), 7));
    running.harness.advance(ms(0));
    assert!(!running.harness.has_class(&row(1), "a-shake-x"));
    assert_settles_to_zero_frames(&mut running.harness);
}

// ---- Bluetooth devices ----------------------------------------------------------------------

#[test]
fn a_device_breathes_while_connecting_then_seals_and_its_battery_sweeps_in() {
    let mut running = center(|_| {});
    press(&mut running.harness, "[*|aria-label=\"Bluetooth details\"]");
    running.harness.advance(ms(50));
    // The pane was just opened: the connected Buds' battery counts up in step with its sweep.
    let buds = format!("{} .ds-settings-row-figure", row(1));
    let start = figure(&running.harness, &buds).unwrap_or(0);
    assert!(start < 80, "the battery sweeps in from empty: {start}");
    assert_eq!(
        running
            .harness
            .attr(&format!("{} .ds-settings-row-glyph", row(1)), "data-disc")
            .as_deref(),
        Some("on")
    );
    assert_settles_to_zero_frames(&mut running.harness);
    assert_eq!(figure(&running.harness, &buds), Some(80));
    // The keyboard connects: its glyph breathes after the grace.
    running.feeds.bluetooth.send_modify(|bt| {
        bt.devices[1].connection = BtConnection::Connecting;
        bt.connect = Attempts {
            now: Attempt::Running(OpStamp(2)),
            minted: OpStamp(2),
        };
    });
    let glyph = format!("{} .ds-settings-row-glyph", row(2));
    let harness = &mut running.harness;
    harness.advance(ms(400));
    assert_eq!(
        harness.attr(&glyph, "data-pending").as_deref(),
        Some("step")
    );
    assert_eq!(
        harness.count(&format!("{} .ds-spinner", row(2))),
        0,
        "the glyph breathes; no spinner"
    );
    harness.advance(ms(10_000));
    assert_eq!(
        harness.attr(&glyph, "data-pending").as_deref(),
        Some("still"),
        "held still at the cap"
    );
    assert_settles_to_zero_frames(harness);
    // Connected with a battery: the disc seals and the battery arrives, sweeping in.
    running.feeds.bluetooth.send_modify(|bt| {
        bt.devices[1].connection = BtConnection::Connected;
        bt.devices[1].battery = BtBattery::Level(Percent(55));
        bt.connect.now = Attempt::Quiet;
    });
    let harness = &mut running.harness;
    harness.advance(ms(0));
    assert!(
        harness
            .attr(&glyph, "class")
            .unwrap_or_default()
            .contains("a-seal-out")
    );
    let keys = format!("{} .ds-settings-row-figure", row(2));
    assert!(figure(harness, &keys).unwrap_or(0) < 55);
    assert_settles_to_zero_frames(harness);
    assert_eq!(figure(harness, &keys), Some(55));
    assert_eq!(harness.attr(&glyph, "data-disc").as_deref(), Some("on"));
}
