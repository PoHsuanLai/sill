//! The status glyphs on the real surfaces: the fake services, mapped by `status::glyph`, drive
//! the status glyphs in the real `Bar`'s Wi-Fi, battery, volume and Bluetooth items and in the
//! real `ControlCenter`'s Wi-Fi and Bluetooth discs, Battery header and Sound level. Each change
//! ends at 0 frames (design/26 R3), on the virtual clock.

use crate::cc_support;

use cc_support::{fake, live};
use std::time::Duration;

use ds_harness::harness::{assert_settles_to_zero_frames, settle_until};
use ds_harness::{ClassPresence, Driver, Harness, Query, Viewport};
use sill_bar::Bar;
use sill_model::attempt::{Attempt, Attempts, OpStamp};
use sill_model::battery::{Battery, Charge};
use sill_model::bluetooth::BtConnection;
use sill_model::network::{Connectivity, Link, Network};
use sill_model::units::{Percent, Presence};
use sill_model::volume::{Mute, Volume};
use sill_model::wifi::Radio;
use sill_settings::{InMenuBar, ShellFile};
use sill_ui_kit::harness::SurfaceHarness;
use sill_ui_kit::links::ControlCenterLink;

const BAR: Viewport = Viewport {
    width: 1280,
    height: 40,
    scale_percent: 100,
};

/// The real bar on the fake services, with its Bluetooth item shown
/// (`control_center.menu_bar_bluetooth`), settled.
fn bar() -> (Harness, fake::Feeds) {
    let mut file = ShellFile::default();
    file.control_center.menu_bar_bluetooth = InMenuBar::Show;
    let mut run = SurfaceHarness::<Bar>::setup(ControlCenterLink::new(), ())
        .settings(file)
        .viewport(BAR)
        .start();
    settle_until(&mut run.harness, |h| h.count(&glyph("Bluetooth")) == 1);
    assert_settles_to_zero_frames(&mut run.harness);
    (run.harness, run.feeds)
}

/// The control center on the fake services, settled.
fn center() -> live::Running {
    let (services, feeds) = fake::services();
    let mut running = live::start(services, feeds);
    settle_until(&mut running.harness, |h| h.count(DISC_WIFI) == 1);
    running
}

/// The status glyph in the bar item named `label`.
fn glyph(label: &str) -> String {
    format!("[*|aria-label={label}] .ds-status-glyph")
}

/// Whether `part` of the glyph at `root` shows `lit`.
fn lit(harness: &Harness, root: &str, part: &str) -> bool {
    harness
        .attr(&format!("{root} [*|data-part={part}]"), "data-show")
        .as_deref()
        == Some("lit")
}

/// How many of the Wi-Fi fan's layers are lit.
fn lit_arcs(harness: &Harness, root: &str) -> usize {
    ["dot", "arc-1", "arc-2", "arc-3"]
        .iter()
        .filter(|part| lit(harness, root, part))
        .count()
}

/// Whether the speaker's `part` at `root` is on (the bar glyph's and the level's alike).
fn on(harness: &Harness, root: &str, part: &str) -> bool {
    harness
        .attr(&format!("{root} [*|data-part={part}]"), "data-on")
        .as_deref()
        == Some("on")
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

fn joining() -> Network {
    network(
        Link::Connecting,
        Connectivity::Unknown,
        Attempt::Running(OpStamp(1)),
    )
}

fn home(strength: u16) -> Link {
    Link::Wifi {
        ssid: "home".to_owned(),
        strength: Percent(strength),
    }
}

fn battery(level: u16, charge: Charge) -> Battery {
    Battery {
        presence: Presence::Present,
        level: Percent(level),
        charge,
        ..Battery::default()
    }
}

// ---- The bar -------------------------------------------------------------------------------

const WIFI: &str = "[*|aria-label=Network] .ds-status-glyph";

/// With its appearance pinned, nothing off the harness's thread is left to wake the settled
/// bar, however much wall time passes. The live appearance's portal replies woke it 5 to 15 ms
/// of wall time after a settle, which a loaded machine stretched into the settle's quiet window.
#[test]
fn the_settled_bar_stays_quiet_on_the_wall_clock() {
    let (mut harness, _feeds) = bar();
    let wakes = harness.wakes();
    std::thread::sleep(Duration::from_millis(200));
    harness.advance(Duration::ZERO);
    assert_eq!(harness.wakes(), wakes, "something off-thread woke the bar");
}

#[test]
fn the_bar_joins_by_searching_never_wifi_off_then_fills_and_grows_the_badge() {
    let (mut harness, feeds) = bar();
    assert_eq!(lit_arcs(&harness, WIFI), 4, "the fake joins at 72 %");
    // Connecting is Joining with the service's stamp, never the slash of "Wi-Fi off".
    feeds.network.send_replace(joining());
    settle_until(&mut harness, |h| lit_arcs(h, WIFI) == 1);
    assert_eq!(harness.count(&format!("{WIFI} [*|data-part=slash]")), 0);
    feeds
        .network
        .send_replace(network(home(50), Connectivity::Full, Attempt::Quiet));
    settle_until(&mut harness, |h| lit_arcs(h, WIFI) == 3);
    assert_settles_to_zero_frames(&mut harness);
    feeds
        .network
        .send_replace(network(home(50), Connectivity::Portal, Attempt::Quiet));
    settle_until(&mut harness, |h| lit(h, WIFI, "badge"));
    assert_settles_to_zero_frames(&mut harness);
    feeds
        .network
        .send_replace(network(Link::Wired, Connectivity::Full, Attempt::Quiet));
    settle_until(&mut harness, |h| h.count(WIFI) == 0);
    assert_eq!(
        harness.count("[*|aria-label=Network] svg.ds-ic"),
        1,
        "a wire stays a plain glyph"
    );
    assert_settles_to_zero_frames(&mut harness);
}

#[test]
fn the_bar_does_not_shake_for_a_failed_join_and_slashes_with_the_radio_off() {
    let (mut harness, feeds) = bar();
    let failed = network(
        Link::Disconnected,
        Connectivity::Unknown,
        Attempt::Failed(OpStamp(2)),
    );
    feeds.network.send_replace(failed);
    harness.advance(Duration::from_millis(60));
    assert_eq!(
        harness.has_class(WIFI, "a-shake-x"),
        ClassPresence::Absent,
        "only a failed secure entry shakes (quire design/30 section 1.3)"
    );
    assert_settles_to_zero_frames(&mut harness);
    feeds.wifi.send_modify(|wifi| wifi.radio = Radio::Off);
    settle_until(&mut harness, |h| {
        h.count(&format!("{WIFI} [*|data-part=slash]")) == 1
    });
    assert_settles_to_zero_frames(&mut harness);
}

#[test]
fn the_bar_battery_turns_low_at_the_key_shows_its_power_and_hides_when_absent() {
    let (mut harness, feeds) = bar();
    let item = glyph("Battery");
    let tone = |h: &Harness| h.attr(&format!("{item} [*|data-part=fill]"), "data-tone");
    feeds.battery.send_replace(battery(18, Charge::Discharging));
    settle_until(&mut harness, |h| tone(h).as_deref() == Some("low"));
    assert_settles_to_zero_frames(&mut harness);
    feeds.battery.send_replace(battery(18, Charge::Charging));
    settle_until(&mut harness, |h| lit(h, &item, "bolt"));
    assert_eq!(
        tone(&harness).as_deref(),
        Some("normal"),
        "charging is never low"
    );
    assert_settles_to_zero_frames(&mut harness);
    feeds.battery.send_replace(battery(100, Charge::Full));
    settle_until(&mut harness, |h| lit(h, &item, "plug"));
    assert_settles_to_zero_frames(&mut harness);
    feeds.battery.send_replace(Battery::default());
    settle_until(&mut harness, |h| h.count("[*|aria-label=Battery]") == 0);
    assert_settles_to_zero_frames(&mut harness);
}

#[test]
fn the_bar_volume_keeps_the_slash_at_zero_and_its_waves_follow() {
    let (mut harness, feeds) = bar();
    let item = glyph("Volume");
    assert!(
        on(&harness, &item, "wave-2") && !on(&harness, &item, "wave-3"),
        "40 %: two waves"
    );
    feeds.volume.send_modify(|v| v.level = Percent(0));
    settle_until(&mut harness, |h| on(h, &item, "slash"));
    assert!(!on(&harness, &item, "wave-1"), "no waves at zero");
    assert_settles_to_zero_frames(&mut harness);
    feeds.volume.send_replace(Volume {
        presence: Presence::Present,
        level: Percent(90),
        mute: Mute::Audible,
        sink: "Speakers".to_owned(),
    });
    settle_until(&mut harness, |h| {
        on(h, &item, "wave-3") && !on(h, &item, "slash")
    });
    assert_settles_to_zero_frames(&mut harness);
}

#[test]
fn the_bar_bluetooth_shows_its_dots_once_connected() {
    let (mut harness, feeds) = bar();
    let item = glyph("Bluetooth");
    feeds.bluetooth.send_modify(|bt| {
        bt.devices[0].connection = BtConnection::Connecting;
        bt.connect = Attempts {
            now: Attempt::Running(OpStamp(1)),
            minted: OpStamp(1),
        };
    });
    settle_until(&mut harness, |h| !lit(h, &item, "dots"));
    feeds.bluetooth.send_modify(|bt| {
        bt.devices[0].connection = BtConnection::Connected;
        bt.connect.now = Attempt::Quiet;
    });
    settle_until(&mut harness, |h| lit(h, &item, "dots"));
    assert_settles_to_zero_frames(&mut harness);
}

// ---- The control center --------------------------------------------------------------------

const DISC_WIFI: &str = ".ds-module-disc > .ds-status-glyph[*|data-kind=wifi]";
const DISC_BT: &str = ".ds-module-disc > .ds-status-glyph[*|data-kind=bluetooth]";
const BATTERY_HEAD: &str = ".ds-module-panel-glyph > .ds-status-glyph[*|data-kind=battery]";
const SOUND_HEAD: &str = ".ds-module-panel-glyph > .ds-status-glyph[*|data-kind=volume]";
/// The Sound module's level (Display's is the other `.ds-slider`).
const SOUND: &str = ".ds-slider[*|aria-label=\"Built-in Audio Analog Stereo\"]";
/// The glyph in the knob (the `Mac` look carries it there, quire design/29 section 16).
const LEVEL: &str = ".ds-slider[*|aria-label=\"Built-in Audio Analog Stereo\"] .ds-slider-thumb";

/// The battery header's drawn fill width, 0 when there is none.
fn fill(harness: &Harness) -> f32 {
    harness
        .attr(&format!("{BATTERY_HEAD} [*|data-part=fill] rect"), "width")
        .and_then(|value| value.parse::<f32>().ok())
        .unwrap_or(0.0)
}

#[test]
fn the_centers_battery_stands_at_its_level_when_opened() {
    let (services, feeds) = fake::services();
    let mut running = live::start(services, feeds);
    let harness = &mut running.harness;
    settle_until(harness, |h| h.count(BATTERY_HEAD) == 1);
    // Nothing sweeps in on first show (quire design/30 section 1.3).
    let start = fill(harness);
    harness.advance(Duration::from_millis(200));
    assert_settles_to_zero_frames(harness);
    let full = fill(harness);
    assert!(
        start > 0.0 && (start - full).abs() < 0.01,
        "{start} then {full}"
    );
}

#[test]
fn the_wifi_disc_searches_while_joining_then_fills() {
    let mut running = center();
    let harness = &mut running.harness;
    assert_eq!(lit_arcs(harness, DISC_WIFI), 4, "joined at 72 %");
    assert_settles_to_zero_frames(harness);
    running.feeds.network.send_replace(joining());
    settle_until(harness, |h| lit_arcs(h, DISC_WIFI) == 1);
    assert_eq!(
        harness.count(&format!("{DISC_WIFI} [*|data-part=slash]")),
        0,
        "joining is not off (G300)"
    );
    running
        .feeds
        .network
        .send_replace(network(home(50), Connectivity::Full, Attempt::Quiet));
    settle_until(harness, |h| lit_arcs(h, DISC_WIFI) == 3);
    assert_settles_to_zero_frames(harness);
    running
        .feeds
        .wifi
        .send_modify(|wifi| wifi.radio = Radio::Off);
    settle_until(harness, |h| {
        h.count(&format!("{DISC_WIFI} [*|data-part=slash]")) == 1
    });
    assert_settles_to_zero_frames(harness);
}

#[test]
fn the_bluetooth_disc_seals_when_a_device_connects() {
    let mut running = center();
    let harness = &mut running.harness;
    assert!(
        lit(harness, DISC_BT, "dots"),
        "the fake's buds are connected"
    );
    assert_settles_to_zero_frames(harness);
    running.feeds.bluetooth.send_modify(|bt| {
        bt.devices[0].connection = BtConnection::Connecting;
        bt.connect = Attempts {
            now: Attempt::Running(OpStamp(1)),
            minted: OpStamp(1),
        };
    });
    settle_until(harness, |h| !lit(h, DISC_BT, "dots"));
    running.feeds.bluetooth.send_modify(|bt| {
        bt.devices[0].connection = BtConnection::Connected;
        bt.connect.now = Attempt::Quiet;
    });
    settle_until(harness, |h| lit(h, DISC_BT, "dots"));
    assert_settles_to_zero_frames(harness);
}

#[test]
fn the_sound_level_and_its_header_read_the_bars_volume_state() {
    let mut running = center();
    let harness = &mut running.harness;
    assert!(
        on(harness, LEVEL, "wave-2") && !on(harness, LEVEL, "wave-3"),
        "40 %: two waves"
    );
    assert_eq!(
        harness.count(SOUND_HEAD),
        1,
        "the bar item's glyph heads it"
    );
    assert_settles_to_zero_frames(harness);
    running.feeds.volume.send_modify(|v| v.mute = Mute::Muted);
    settle_until(harness, |h| {
        on(h, LEVEL, "slash") && on(h, SOUND_HEAD, "slash")
    });
    assert!(!on(harness, LEVEL, "wave-1"), "muted draws no waves");
    assert_eq!(
        harness.attr(SOUND, "aria-valuenow").as_deref(),
        Some("40"),
        "the capsule still shows the level"
    );
    assert_settles_to_zero_frames(harness);
    running.feeds.volume.send_modify(|v| {
        v.mute = Mute::Audible;
        v.level = Percent(0);
    });
    settle_until(harness, |h| {
        h.attr(SOUND, "aria-valuenow").as_deref() == Some("0")
    });
    assert!(on(harness, LEVEL, "slash"), "zero keeps the slash");
    assert_settles_to_zero_frames(harness);
}
