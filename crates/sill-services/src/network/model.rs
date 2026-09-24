//! The network's state and its one reducer.

use serde::{Deserialize, Serialize};

use crate::units::Percent;

/// What the bar's network item draws.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Network {
    pub link: Link,
    pub connectivity: Connectivity,
}

/// The primary connection.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum Link {
    #[default]
    Disconnected,
    /// Activating, not yet up.
    Connecting,
    Wired,
    Wifi {
        ssid: String,
        strength: Percent,
    },
    /// A VPN, a bridge, a modem: shown as connected without a signal.
    Other,
}

/// NetworkManager's `Connectivity`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Connectivity {
    #[default]
    Unknown,
    None,
    /// Behind a captive portal.
    Portal,
    Limited,
    Full,
}

impl Connectivity {
    /// NetworkManager's `NMConnectivityState` number; anything else is `Unknown`.
    pub fn from_nm(state: u32) -> Connectivity {
        match state {
            1 => Connectivity::None,
            2 => Connectivity::Portal,
            3 => Connectivity::Limited,
            4 => Connectivity::Full,
            _ => Connectivity::Unknown,
        }
    }
}

/// One change NetworkManager reported.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum NetworkEvent {
    /// The primary connection changed.
    Link(Link),
    /// The active access point's strength changed; ignored unless the link is Wi-Fi.
    Strength(Percent),
    Connectivity(Connectivity),
    /// NetworkManager left the bus.
    Gone,
}

/// `network` after `event`.
pub fn reduce_network(network: Network, event: NetworkEvent) -> Network {
    match event {
        NetworkEvent::Link(link) => Network { link, ..network },
        NetworkEvent::Strength(strength) => match network.link {
            Link::Wifi { ssid, .. } => Network {
                link: Link::Wifi {
                    ssid,
                    strength: Percent(strength.0.min(100)),
                },
                ..network
            },
            _ => network,
        },
        NetworkEvent::Connectivity(connectivity) => Network {
            connectivity,
            ..network
        },
        NetworkEvent::Gone => Network::default(),
    }
}

#[cfg(test)]
mod tests {
    use super::{Connectivity, Link, Network, NetworkEvent, reduce_network};
    use crate::units::Percent;

    fn wifi(strength: u16) -> Link {
        Link::Wifi {
            ssid: "home".to_owned(),
            strength: Percent(strength),
        }
    }

    fn net(link: Link, connectivity: Connectivity) -> Network {
        Network { link, connectivity }
    }

    #[test]
    fn events_reduce_to_the_expected_network() {
        let cases: Vec<(&str, Network, NetworkEvent, Network)> = vec![
            (
                "wifi comes up",
                Network::default(),
                NetworkEvent::Link(wifi(70)),
                net(wifi(70), Connectivity::Unknown),
            ),
            (
                "strength moves on wifi",
                net(wifi(70), Connectivity::Full),
                NetworkEvent::Strength(Percent(40)),
                net(wifi(40), Connectivity::Full),
            ),
            (
                "strength is clamped",
                net(wifi(70), Connectivity::Full),
                NetworkEvent::Strength(Percent(250)),
                net(wifi(100), Connectivity::Full),
            ),
            (
                "strength is ignored on a wire",
                net(Link::Wired, Connectivity::Full),
                NetworkEvent::Strength(Percent(40)),
                net(Link::Wired, Connectivity::Full),
            ),
            (
                "connectivity alone",
                net(Link::Wired, Connectivity::Unknown),
                NetworkEvent::Connectivity(Connectivity::Portal),
                net(Link::Wired, Connectivity::Portal),
            ),
            (
                "disconnect keeps the last connectivity until NM says otherwise",
                net(wifi(70), Connectivity::Full),
                NetworkEvent::Link(Link::Disconnected),
                net(Link::Disconnected, Connectivity::Full),
            ),
            (
                "networkmanager gone",
                net(wifi(70), Connectivity::Full),
                NetworkEvent::Gone,
                Network::default(),
            ),
        ];
        for (name, before, event, want) in cases {
            assert_eq!(reduce_network(before, event), want, "{name}");
        }
    }

    #[test]
    fn nm_connectivity_numbers_map() {
        const CASES: &[(u32, Connectivity)] = &[
            (0, Connectivity::Unknown),
            (1, Connectivity::None),
            (2, Connectivity::Portal),
            (3, Connectivity::Limited),
            (4, Connectivity::Full),
            (9, Connectivity::Unknown),
        ];
        for &(n, want) in CASES {
            assert_eq!(Connectivity::from_nm(n), want, "{n}");
        }
    }
}
