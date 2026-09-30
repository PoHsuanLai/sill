//! Row details on the real control center (design/26 5.2.2, 5.2.3): the networks and devices
//! panes' row phases from the services' stamps (`Network.join`,
//! `Bluetooth.connect`), driven by the fake services on `Clock::Virtual`. A pending row spins at
//! once and for as long as its operation runs; every other state change ends at 0 frames (R3). Nothing here reaches a real radio.

mod cc_support;

use cc_support::details::{center, figure, ms, open, press};
use cc_support::fake;

use ds_harness::Harness;
use ds_harness::harness::assert_settles_to_zero_frames;
use sill_model::attempt::{Attempt, Attempts, OpStamp};
use sill_model::bluetooth::{BtBattery, BtConnection};
use sill_model::network::{Connectivity, Link, Network};
use sill_model::units::Percent;
use sill_model::wifi::{AccessPoint, Association, Security, Ssid, Wifi};

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

/// The spinner that holds a row's trailing slot while its operation runs.
const PENDING: &str = ".ds-row-trailing[*|data-mark=busy]";

fn row(n: usize) -> String {
    format!(".ds-list > .ds-list-item:nth-child({n}) > .ds-row")
}

fn title(harness: &Harness, n: usize) -> Option<String> {
    harness.text_of(&format!("{} .ds-row-title", row(n)))
}

#[test]
fn a_network_joins_with_a_spinner_where_its_lock_was_then_takes_the_disc() {
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
    // The spinner takes the lock's place at once, with no grace.
    assert_eq!(harness.count(&format!("{} {PENDING}", row(2))), 1);
    assert_eq!(harness.count(PENDING), 1, "only the joining row");
    harness.advance(ms(10_000));
    assert_eq!(
        harness.count(&format!("{} {PENDING}", row(2))),
        1,
        "it keeps turning for as long as the join runs"
    );
    // It lands: office moves to the top, keeps its row, and its disc is on.
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
    let glyph = format!("{} .ds-row-leading", row(1));
    assert_eq!(harness.attr(&glyph, "data-disc").as_deref(), Some("on"));
    assert_eq!(
        harness.attr(&row(1), "aria-pressed"),
        None,
        "no check on the network in use: the disc says it"
    );
    assert_eq!(
        harness.count(&format!("{} .ds-row-trailing svg.ds-ic", row(1))),
        1,
        "the lock is back"
    );
    assert_eq!(harness.count(PENDING), 0);
    assert_eq!(
        harness
            .attr(&format!("{} .ds-row-leading", row(2)), "data-disc")
            .as_deref(),
        Some("off")
    );
    assert_settles_to_zero_frames(harness);
}

#[test]
fn a_failed_join_says_so_and_does_not_shake() {
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
    // A join already running when the pane opens is pending at once.
    running.harness.advance(ms(500));
    assert_eq!(running.harness.count(&format!("{} {PENDING}", row(1))), 1);
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
    assert!(
        !harness.has_class(&row(1), "a-shake-x"),
        "only a failed secure entry shakes"
    );
    assert_eq!(
        harness
            .text_of(&format!("{} .ds-row-detail", row(1)))
            .as_deref(),
        Some("Couldn’t connect")
    );
    assert_eq!(harness.count(PENDING), 0);
    assert_settles_to_zero_frames(harness);
}

// ---- Bluetooth devices ----------------------------------------------------------------------

#[test]
fn a_connecting_device_spins_and_a_connected_one_shows_its_battery_at_once() {
    let mut running = center(|_| {});
    press(&mut running.harness, "[*|aria-label=\"Bluetooth details\"]");
    running.harness.advance(ms(50));
    // The connected Buds' battery is its level from the first frame: nothing sweeps in.
    let buds = format!("{} .ds-row-figure", row(1));
    assert_eq!(figure(&running.harness, &buds), Some(80));
    assert_eq!(
        running
            .harness
            .attr(&format!("{} .ds-row-leading", row(1)), "data-disc")
            .as_deref(),
        Some("on")
    );
    assert_settles_to_zero_frames(&mut running.harness);
    // The keyboard connects: a spinner takes its trailing slot at once.
    running.feeds.bluetooth.send_modify(|bt| {
        bt.devices[1].connection = BtConnection::Connecting;
        bt.connect = Attempts {
            now: Attempt::Running(OpStamp(2)),
            minted: OpStamp(2),
        };
    });
    let glyph = format!("{} .ds-row-leading", row(2));
    let harness = &mut running.harness;
    harness.advance(ms(0));
    assert_eq!(harness.count(&format!("{} {PENDING}", row(2))), 1);
    // Connected with a battery: the disc is on and the battery shows its level.
    running.feeds.bluetooth.send_modify(|bt| {
        bt.devices[1].connection = BtConnection::Connected;
        bt.devices[1].battery = BtBattery::Level(Percent(55));
        bt.connect.now = Attempt::Quiet;
    });
    let harness = &mut running.harness;
    harness.advance(ms(0));
    let keys = format!("{} .ds-row-figure", row(2));
    assert_eq!(figure(harness, &keys), Some(55));
    assert_eq!(harness.count(PENDING), 0);
    assert_settles_to_zero_frames(harness);
    assert_eq!(harness.attr(&glyph, "data-disc").as_deref(), Some("on"));
}
