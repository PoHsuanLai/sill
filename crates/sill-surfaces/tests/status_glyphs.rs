//! The status glyphs on the real surfaces: the fake services, mapped by `glyph_state`, drive
//! the status glyphs in the real `Bar`'s Wi-Fi, battery, volume and Bluetooth items and in the
//! real `ControlCenter`'s Wi-Fi and Bluetooth discs, Battery header and Sound level. Each change
//! ends at 0 frames (design/26 R3), on the virtual clock.

mod cc_support;

use cc_support::{fake, live};
use std::cell::RefCell;
use std::time::Duration;

use dioxus::prelude::*;
use ds_native::harness::{assert_settles_to_zero_frames, settle_until};
use ds_native::{Clock, Harness, HarnessConfig, Viewport};
use sill_model::attempt::{Attempt, Attempts, OpStamp};
use sill_model::battery::{Battery, Charge};
use sill_model::bluetooth::BtConnection;
use sill_model::network::{Connectivity, Link, Network};
use sill_model::units::{Percent, Presence};
use sill_model::volume::{Mute, Volume};
use sill_model::wifi::Radio;
use sill_services::Services;
use sill_settings::{InMenuBar, SettingsFeed, ShellFile};
use sill_surfaces::Bar;
use sill_surfaces::hooks::PinnedAppearance;

thread_local! {
    static SERVICES: RefCell<Option<Services>> = const { RefCell::new(None) };
}

/// The bar with its Bluetooth item shown (`control_center.menu_bar_bluetooth`).
fn bar_app() -> Element {
    let services = SERVICES.with(|s| s.borrow().clone());
    let Some(services) = services else {
        return rsx! {};
    };
    use_context_provider(|| services);
    // Not the desktop's portal and appearance file: they answer off-thread.
    use_context_provider(PinnedAppearance::default);
    use_context_provider(|| {
        let mut file = ShellFile::default();
        file.control_center.menu_bar_bluetooth = InMenuBar::Show;
        SettingsFeed::fixed(file)
    });
    rsx! { Bar { output: fake::output() } }
}

const BAR: Viewport = Viewport {
    width: 1280,
    height: 40,
    scale_percent: 100,
};

/// The real bar on the fake services, settled.
fn bar() -> (Harness, fake::Feeds) {
    let (services, feeds) = fake::services();
    SERVICES.with(|s| *s.borrow_mut() = Some(services));
    let mut harness =
        Harness::with_config(bar_app, HarnessConfig::new(BAR).with_clock(Clock::Virtual));
    settle_until(&mut harness, |h| h.count(&glyph("Bluetooth")) == 1);
    assert_settles_to_zero_frames(&mut harness);
    (harness, feeds)
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
fn the_bar_shakes_once_for_a_failed_join_and_slashes_with_the_radio_off() {
    let (mut harness, feeds) = bar();
    let failed = network(
        Link::Disconnected,
        Connectivity::Unknown,
        Attempt::Failed(OpStamp(2)),
    );
    feeds.network.send_replace(failed.clone());
    settle_until(&mut harness, |h| h.has_class(WIFI, "a-shake-x"));
    assert_settles_to_zero_frames(&mut harness);
    feeds.network.send_replace(failed);
    harness.advance(Duration::from_millis(60));
    assert!(
        !harness.has_class(WIFI, "a-shake-x"),
        "the same stamp shook again"
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
fn the_bar_bluetooth_breathes_while_connecting_and_seals_when_connected() {
    let (mut harness, feeds) = bar();
    let item = glyph("Bluetooth");
    feeds.bluetooth.send_modify(|bt| {
        bt.devices[0].connection = BtConnection::Connecting;
        bt.connect = Attempts {
            now: Attempt::Running(OpStamp(1)),
            minted: OpStamp(1),
        };
    });
    settle_until(&mut harness, |h| {
        matches!(
            h.attr(&item, "data-pending").as_deref(),
            Some("high" | "low")
        )
    });
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
/// The Sound module's level (Display's is the other `.ds-level`).
const SOUND: &str = ".ds-level[*|aria-label=\"Built-in Audio Analog Stereo\"]";
/// The glyph before the capsule (`CapsuleKnob` leads with it, design/29).
const LEVEL: &str = ".ds-level[*|aria-label=\"Built-in Audio Analog Stereo\"] .ds-level-lead";

/// The battery header's drawn fill width, 0 when there is none.
fn fill(harness: &Harness) -> f32 {
    harness
        .attr(&format!("{BATTERY_HEAD} [*|data-part=fill] rect"), "width")
        .and_then(|value| value.parse::<f32>().ok())
        .unwrap_or(0.0)
}

#[test]
fn the_centers_battery_sweeps_in_when_opened() {
    let (services, feeds) = fake::services();
    let mut running = live::start(services, feeds);
    let harness = &mut running.harness;
    settle_until(harness, |h| h.count(BATTERY_HEAD) == 1);
    let start = fill(harness);
    harness.advance(Duration::from_millis(200));
    let midway = fill(harness);
    assert_settles_to_zero_frames(harness);
    let full = fill(harness);
    assert!(
        start < midway && midway < full,
        "the fill sweeps in from empty: {start} {midway} {full}"
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
