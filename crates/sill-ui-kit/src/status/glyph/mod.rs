//! The service values as quire's status glyph states (design/26 section 5.1), pure: which Wi-Fi,
//! battery, Bluetooth and volume state each item hands its glyph. The glyph decides the moment
//! from its own table; these only say what is true.

use ds::components::content::status::battery_state::{BatteryPower, BatteryState, LowAt};
use ds::components::content::status::bluetooth_state::BluetoothState;
use ds::components::content::status::volume::{VolumeState, VolumeWaves};
use ds::components::content::status::wifi_state::{WifiBars, WifiReach, WifiState};
use ds::motion::detail::stamp::EventStamp;
use ds::prelude::*;
use sill_model::attempt::{Attempt, OpStamp};
use sill_model::battery::{Battery, Charge};
use sill_model::bluetooth::{Bluetooth, BtConnection, BtPower};
use sill_model::network::{Connectivity, Link, Network};
use sill_model::units::Presence;
use sill_model::volume::{Mute, Volume};
use sill_model::wifi::Radio;
use sill_settings::Percent as KeyPercent;

/// What the network item draws: the Wi-Fi glyph, or a plain glyph for a link that is not Wi-Fi.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkGlyph {
    /// Wi-Fi, joined or not.
    Wifi(WifiState),
    /// A wire (`Ethernet`) or another link (`Link`).
    Plain(Icon),
}

impl From<NetworkGlyph> for IconSource {
    /// The Wi-Fi glyph as a layered status glyph; a wire or another link as its plain glyph.
    fn from(glyph: NetworkGlyph) -> Self {
        match glyph {
            NetworkGlyph::Wifi(state) => IconSource::Status(StatusState::Wifi(state)),
            NetworkGlyph::Plain(icon) => IconSource::Glyph(icon),
        }
    }
}

/// A service's op stamp as the glyph's event stamp.
fn stamp(op: OpStamp) -> EventStamp {
    EventStamp(op.0)
}

/// A service percentage as thousandths.
fn thousandths(percent: u16) -> Fraction {
    Fraction(percent.saturating_mul(10))
}

/// The network item's glyph. Joining carries the service's join stamp (one per attempt);
/// connectivity None, Portal or Limited is the "!" badge; disconnected is Off with the radio off,
/// the last failure while it stands, else Idle.
pub fn network_glyph(network: &Network, radio: Radio) -> NetworkGlyph {
    match (&network.link, network.join.now, radio) {
        // Moving to another Wi-Fi network while joined: joining, as the reference's menu bar
        // does. A wired primary keeps its own icon.
        (Link::Connecting | Link::Wifi { .. }, Attempt::Running(op), _) => {
            NetworkGlyph::Wifi(WifiState::Joining(stamp(op)))
        }
        // The reducer always stamps a join; this is a join it has not seen start.
        (Link::Connecting, Attempt::Quiet | Attempt::Failed(_), _) => {
            NetworkGlyph::Wifi(WifiState::Joining(stamp(network.join.minted)))
        }
        (Link::Wifi { strength, .. }, _, _) => NetworkGlyph::Wifi(WifiState::Joined {
            bars: WifiBars::of(thousandths(strength.0)),
            reach: reach(network.connectivity),
        }),
        (Link::Wired, _, _) => NetworkGlyph::Plain(Icon::Ethernet),
        (Link::Other, _, _) => NetworkGlyph::Plain(Icon::Link),
        (Link::Disconnected, _, Radio::Off | Radio::Blocked) => NetworkGlyph::Wifi(WifiState::Off),
        (Link::Disconnected, Attempt::Failed(op), Radio::On) => {
            NetworkGlyph::Wifi(WifiState::Failed(stamp(op)))
        }
        (Link::Disconnected, Attempt::Quiet | Attempt::Running(_), Radio::On) => {
            NetworkGlyph::Wifi(WifiState::Idle)
        }
    }
}

/// Whether a joined network reaches the internet: unknown is not a claim (R15).
fn reach(connectivity: Connectivity) -> WifiReach {
    match connectivity {
        Connectivity::None | Connectivity::Portal | Connectivity::Limited => WifiReach::NoInternet,
        Connectivity::Full | Connectivity::Unknown => WifiReach::Internet,
    }
}

/// The battery item's state, `None` without a battery (the item hides). `low` is
/// `bar.battery_low_percent`.
pub fn battery_glyph(battery: &Battery, low: KeyPercent) -> Option<BatteryState> {
    let power = match battery.charge {
        Charge::Charging => BatteryPower::Charging,
        Charge::Full | Charge::Idle => BatteryPower::Held,
        Charge::Discharging | Charge::Empty | Charge::Unknown => BatteryPower::Battery,
    };
    match battery.presence {
        Presence::Absent => None,
        Presence::Present => Some(BatteryState {
            level: thousandths(battery.level.0),
            power,
            low_at: LowAt(thousandths(u16::from(low.0))),
        }),
    }
}

/// The volume item's state. Level 0 is Muted, so the slash stays (as the reference draws it).
pub fn volume_glyph(volume: &Volume) -> VolumeState {
    match (volume.presence, volume.mute, volume.level.0) {
        (Presence::Absent, _, _) => VolumeState::NoDevice,
        (Presence::Present, Mute::Muted, _) | (Presence::Present, Mute::Audible, 0) => {
            VolumeState::Muted
        }
        (Presence::Present, Mute::Audible, level) => {
            VolumeState::Heard(VolumeWaves::of(thousandths(level)))
        }
    }
}

/// The Bluetooth item's state: off without an adapter or with the radio off, then a connect
/// under way, then any device connected, then the last failed connect, else on.
pub fn bluetooth_glyph_state(bluetooth: &Bluetooth) -> BluetoothState {
    let connected = bluetooth
        .devices
        .iter()
        .any(|d| d.connection == BtConnection::Connected);
    match (
        bluetooth.presence,
        bluetooth.power,
        bluetooth.connect.now,
        connected,
    ) {
        (Presence::Absent, _, _, _) | (_, BtPower::Off, _, _) => BluetoothState::Off,
        (Presence::Present, BtPower::On, Attempt::Running(op), _) => {
            BluetoothState::Connecting(stamp(op))
        }
        (Presence::Present, BtPower::On, Attempt::Quiet | Attempt::Failed(_), true) => {
            BluetoothState::Connected
        }
        (Presence::Present, BtPower::On, Attempt::Failed(op), false) => {
            BluetoothState::Failed(stamp(op))
        }
        (Presence::Present, BtPower::On, Attempt::Quiet, false) => BluetoothState::On,
    }
}

#[cfg(test)]
mod tests;
