//! Table tests for `glyph_state`: every row of quire's status glyph table.

use super::{NetworkGlyph, battery_glyph, bluetooth_glyph_state, network_glyph, volume_glyph};
use ds::detail::EventStamp;
use ds::{
    BatteryPower, BatteryState, BluetoothState, Fraction, Icon, LowAt, VolumeState, VolumeWaves,
    WifiBars, WifiReach, WifiState,
};
use sill_model::attempt::{Attempt, Attempts, OpStamp};
use sill_model::battery::{Battery, Charge};
use sill_model::units::{Percent, Presence, Seconds};
use sill_services::{
    BtAddress, BtBattery, BtConnection, BtDevice, BtKind, BtPairing, BtPower, Connectivity, Link,
    Mute, Network, Radio, Volume,
};
use sill_settings::Percent as KeyPercent;

fn net(link: Link, connectivity: Connectivity, now: Attempt) -> Network {
    Network {
        link,
        connectivity,
        join: Attempts {
            now,
            minted: OpStamp(9),
        },
    }
}

fn wifi(strength: u16) -> Link {
    Link::Wifi {
        ssid: "home".to_owned(),
        strength: Percent(strength),
    }
}

fn joined(bars: WifiBars, reach: WifiReach) -> NetworkGlyph {
    NetworkGlyph::Wifi(WifiState::Joined { bars, reach })
}

#[test]
fn the_network_maps_to_the_wifi_glyph() {
    use Attempt::{Failed, Quiet, Running};
    use Connectivity::{Full, Limited, None, Portal, Unknown};
    use WifiBars::{One, Three, Two};
    use WifiReach::{Internet, NoInternet};
    let s = OpStamp;
    let w = NetworkGlyph::Wifi;
    // (name, network, radio, glyph)
    let cases = [
        (
            "joining carries its stamp",
            net(Link::Connecting, Unknown, Running(s(4))),
            Radio::On,
            w(WifiState::Joining(EventStamp(4))),
        ),
        (
            "an unstamped join",
            net(Link::Connecting, Unknown, Quiet),
            Radio::On,
            w(WifiState::Joining(EventStamp(9))),
        ),
        (
            "joined, joining another network (G303)",
            net(wifi(90), Full, Running(s(6))),
            Radio::On,
            w(WifiState::Joining(EventStamp(6))),
        ),
        (
            "wired, joining Wi-Fi keeps the wired icon",
            net(Link::Wired, Full, Running(s(7))),
            Radio::On,
            NetworkGlyph::Plain(Icon::Ethernet),
        ),
        (
            "weak",
            net(wifi(20), Full, Quiet),
            Radio::On,
            joined(One, Internet),
        ),
        (
            "middling",
            net(wifi(50), Full, Quiet),
            Radio::On,
            joined(Two, Internet),
        ),
        (
            "strong",
            net(wifi(90), Full, Quiet),
            Radio::On,
            joined(Three, Internet),
        ),
        (
            "unknown reach is no claim",
            net(wifi(90), Unknown, Quiet),
            Radio::On,
            joined(Three, Internet),
        ),
        (
            "no internet",
            net(wifi(90), None, Quiet),
            Radio::On,
            joined(Three, NoInternet),
        ),
        (
            "a portal",
            net(wifi(50), Portal, Quiet),
            Radio::On,
            joined(Two, NoInternet),
        ),
        (
            "limited",
            net(wifi(20), Limited, Quiet),
            Radio::On,
            joined(One, NoInternet),
        ),
        (
            "idle",
            net(Link::Disconnected, Full, Quiet),
            Radio::On,
            w(WifiState::Idle),
        ),
        (
            "a failed join",
            net(Link::Disconnected, Unknown, Failed(s(5))),
            Radio::On,
            w(WifiState::Failed(EventStamp(5))),
        ),
        (
            "radio off",
            net(Link::Disconnected, Unknown, Failed(s(5))),
            Radio::Off,
            w(WifiState::Off),
        ),
        (
            "radio blocked",
            net(Link::Disconnected, Unknown, Quiet),
            Radio::Blocked,
            w(WifiState::Off),
        ),
        (
            "a wire",
            net(Link::Wired, Full, Quiet),
            Radio::On,
            NetworkGlyph::Plain(Icon::Ethernet),
        ),
        (
            "another link",
            net(Link::Other, Full, Quiet),
            Radio::On,
            NetworkGlyph::Plain(Icon::Link),
        ),
    ];
    for (name, network, radio, want) in cases {
        assert_eq!(network_glyph(&network, radio), want, "{name}");
    }
}

fn battery(level: u16, charge: Charge) -> Battery {
    Battery {
        presence: Presence::Present,
        level: Percent(level),
        charge,
        time_to_empty: Seconds(0),
        time_to_full: Seconds(0),
    }
}

fn state(level: u16, power: BatteryPower, low: u16) -> Option<BatteryState> {
    Some(BatteryState {
        level: Fraction(level),
        power,
        low_at: LowAt(Fraction(low)),
    })
}

#[test]
fn the_battery_maps_to_the_battery_glyph() {
    use BatteryPower::{Battery as OnBattery, Charging, Held};
    let low = KeyPercent(20);
    // (name, battery, state)
    let cases = [
        (
            "discharging",
            battery(64, Charge::Discharging),
            state(640, OnBattery, 200),
        ),
        ("empty", battery(0, Charge::Empty), state(0, OnBattery, 200)),
        (
            "unknown",
            battery(50, Charge::Unknown),
            state(500, OnBattery, 200),
        ),
        (
            "charging",
            battery(30, Charge::Charging),
            state(300, Charging, 200),
        ),
        ("full", battery(100, Charge::Full), state(1000, Held, 200)),
        ("held", battery(80, Charge::Idle), state(800, Held, 200)),
        ("absent", Battery::default(), None),
    ];
    for (name, b, want) in cases {
        assert_eq!(battery_glyph(&b, low), want, "{name}");
    }
    assert_eq!(
        battery_glyph(&battery(64, Charge::Discharging), KeyPercent(35)),
        state(640, OnBattery, 350),
        "the key sets the threshold"
    );
}

#[test]
fn the_volume_maps_to_the_volume_glyph() {
    let v = |level, mute| Volume {
        presence: Presence::Present,
        level: Percent(level),
        mute,
        sink: "Speakers".to_owned(),
    };
    // (name, volume, state)
    let cases = [
        ("no device", Volume::default(), VolumeState::NoDevice),
        ("muted", v(50, Mute::Muted), VolumeState::Muted),
        (
            "zero keeps the slash",
            v(0, Mute::Audible),
            VolumeState::Muted,
        ),
        (
            "low",
            v(20, Mute::Audible),
            VolumeState::Heard(VolumeWaves::One),
        ),
        (
            "half",
            v(50, Mute::Audible),
            VolumeState::Heard(VolumeWaves::Two),
        ),
        (
            "loud",
            v(90, Mute::Audible),
            VolumeState::Heard(VolumeWaves::Three),
        ),
        (
            "boosted",
            v(150, Mute::Audible),
            VolumeState::Heard(VolumeWaves::Three),
        ),
    ];
    for (name, volume, want) in cases {
        assert_eq!(volume_glyph(&volume), want, "{name}");
    }
}

#[test]
fn bluetooth_maps_to_the_bluetooth_glyph() {
    use Attempt::{Failed, Quiet, Running};
    use BtConnection::{Connected, Disconnected};
    let s = OpStamp;
    let device = |connection| BtDevice {
        address: BtAddress("AA".to_owned()),
        name: "Buds".to_owned(),
        kind: BtKind::Headphones,
        connection,
        pairing: BtPairing::Paired,
        battery: BtBattery::Unknown,
    };
    let bt = |power, connection, now| sill_services::Bluetooth {
        presence: Presence::Present,
        power,
        devices: vec![device(connection)],
        connect: Attempts {
            now,
            minted: OpStamp(7),
        },
    };
    // (name, bluetooth, state)
    let cases = [
        (
            "no adapter",
            sill_services::Bluetooth::default(),
            BluetoothState::Off,
        ),
        (
            "radio off",
            bt(BtPower::Off, Disconnected, Running(s(3))),
            BluetoothState::Off,
        ),
        (
            "on",
            bt(BtPower::On, Disconnected, Quiet),
            BluetoothState::On,
        ),
        (
            "connecting",
            bt(BtPower::On, Disconnected, Running(s(3))),
            BluetoothState::Connecting(EventStamp(3)),
        ),
        (
            "connected",
            bt(BtPower::On, Connected, Quiet),
            BluetoothState::Connected,
        ),
        (
            "failed",
            bt(BtPower::On, Disconnected, Failed(s(4))),
            BluetoothState::Failed(EventStamp(4)),
        ),
        (
            "a failure beside a connected device",
            bt(BtPower::On, Connected, Failed(s(4))),
            BluetoothState::Connected,
        ),
    ];
    for (name, bluetooth, want) in cases {
        assert_eq!(bluetooth_glyph_state(&bluetooth), want, "{name}");
    }
}
