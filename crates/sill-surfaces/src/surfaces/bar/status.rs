//! What each status item shows: its glyph and its words, pure over the service values.

use ds::Icon;
use sill_services::{
    Battery, Charge, Connectivity, Link, Minutes, Network, Presence, Remaining, Volume,
};

/// The battery glyph: charging, a warning at 10 % or below, then by level.
pub(crate) fn battery_icon(battery: &Battery) -> Icon {
    match (battery.charge, battery.level.0) {
        (Charge::Charging, _) => Icon::BatteryCharging,
        (Charge::Empty, _) | (_, 0..=10) => Icon::BatteryWarning,
        (_, 11..=35) => Icon::BatteryLow,
        (_, 36..=75) => Icon::BatteryMedium,
        _ => Icon::BatteryFull,
    }
}

/// The volume menu's heading: the sink, or that there is none.
pub(crate) fn volume_heading(volume: &Volume) -> String {
    match volume.presence {
        Presence::Present if volume.sink.is_empty() => "Output".to_owned(),
        Presence::Present => volume.sink.clone(),
        Presence::Absent => "No output device".to_owned(),
    }
}

/// The network menu's two lines: the connection and the connectivity.
pub(crate) fn network_lines(network: &Network) -> [String; 2] {
    let link = match &network.link {
        Link::Disconnected => "Not connected".to_owned(),
        Link::Connecting => "Connecting…".to_owned(),
        Link::Wired => "Wired".to_owned(),
        Link::Wifi { ssid, strength } => format!("Wi-Fi: {ssid} ({}%)", strength.0),
        Link::Other => "Connected".to_owned(),
    };
    let reach = match network.connectivity {
        Connectivity::Unknown => "Internet: unknown",
        Connectivity::None => "No internet",
        Connectivity::Portal => "Sign-in required",
        Connectivity::Limited => "Limited connectivity",
        Connectivity::Full => "Connected to the internet",
    };
    [link, reach.to_owned()]
}

/// The battery menu's two lines: the level and state, and the time left.
pub(crate) fn battery_lines(battery: &Battery) -> [String; 2] {
    let state = match battery.charge {
        Charge::Charging => "charging",
        Charge::Discharging => "on battery",
        Charge::Empty => "empty",
        Charge::Full => "fully charged",
        Charge::Idle => "plugged in",
        Charge::Unknown => "state unknown",
    };
    let left = match battery.remaining() {
        Remaining::Unknown => "Estimating time…".to_owned(),
        Remaining::ToEmpty(m) => format!("{} remaining", hours(m)),
        Remaining::ToFull(m) => format!("{} until full", hours(m)),
    };
    [format!("Battery {}%, {state}", battery.level.0), left]
}

/// `2:05` for 125 minutes.
fn hours(minutes: Minutes) -> String {
    format!("{}:{:02}", minutes.0 / 60, minutes.0 % 60)
}

#[cfg(test)]
mod tests {
    use super::{battery_icon, battery_lines, network_lines, volume_heading};
    use ds::Icon;
    use sill_services::{
        Battery, Charge, Connectivity, Link, Mute, Network, Percent, Presence, Seconds, Volume,
    };

    fn volume(level: u16, mute: Mute) -> Volume {
        Volume {
            presence: Presence::Present,
            level: Percent(level),
            mute,
            sink: "Speakers".to_owned(),
        }
    }

    #[test]
    fn the_volume_heading_names_the_sink() {
        assert_eq!(volume_heading(&volume(5, Mute::Audible)), "Speakers");
        assert_eq!(volume_heading(&Volume::default()), "No output device");
    }

    #[test]
    fn the_network_lines_name_the_link_and_reach() {
        let wifi = |s| Link::Wifi {
            ssid: "home".to_owned(),
            strength: Percent(s),
        };
        let net = |link, connectivity| Network {
            link,
            connectivity,
            ..Network::default()
        };
        assert_eq!(
            network_lines(&net(wifi(70), Connectivity::Portal)),
            [
                "Wi-Fi: home (70%)".to_owned(),
                "Sign-in required".to_owned()
            ]
        );
    }

    #[test]
    fn the_battery_glyph_and_lines_follow_charge() {
        let battery = |level, charge, empty| Battery {
            presence: Presence::Present,
            level: Percent(level),
            charge,
            time_to_empty: Seconds(empty),
            time_to_full: Seconds(0),
        };
        let cases = [
            (battery(50, Charge::Charging, 0), Icon::BatteryCharging),
            (battery(8, Charge::Discharging, 0), Icon::BatteryWarning),
            (battery(30, Charge::Discharging, 0), Icon::BatteryLow),
            (battery(60, Charge::Discharging, 0), Icon::BatteryMedium),
            (battery(100, Charge::Full, 0), Icon::BatteryFull),
        ];
        for (b, want) in cases {
            assert_eq!(battery_icon(&b), want, "{b:?}");
        }
        assert_eq!(
            battery_lines(&battery(64, Charge::Discharging, 7500)),
            [
                "Battery 64%, on battery".to_owned(),
                "2:05 remaining".to_owned()
            ]
        );
    }
}
