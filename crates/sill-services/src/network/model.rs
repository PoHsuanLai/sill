//! The network's state and its one reducer.

use serde::{Deserialize, Serialize};

use crate::attempt::{AttemptStep as Step, Attempts};
use crate::units::Percent;

/// What the bar's network item draws.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Network {
    pub link: Link,
    pub connectivity: Connectivity,
    /// The join attempts: one stamp per entry into `Connecting`, a new one per failed
    /// activation (the bar's Wi-Fi glyph, quire design/26 R4 and R6).
    #[serde(default)]
    pub join: Attempts,
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

/// What a change of link means for the join under way: entering `Connecting` starts one (or
/// is the same one reported again), leaving it for `Disconnected` is a failed activation, and
/// any link that is up lands it.
fn join_step(from: &Link, to: &Link) -> Step {
    match (from, to) {
        (_, Link::Connecting) => Step::Start,
        (Link::Connecting, Link::Disconnected) => Step::Fail,
        (_, Link::Wired | Link::Wifi { .. } | Link::Other) => Step::Land,
        (
            Link::Disconnected | Link::Wired | Link::Wifi { .. } | Link::Other,
            Link::Disconnected,
        ) => Step::Hold,
    }
}

/// `network` after `event`.
pub fn reduce_network(network: Network, event: NetworkEvent) -> Network {
    match event {
        NetworkEvent::Link(link) => Network {
            join: network.join.next(join_step(&network.link, &link)),
            link,
            ..network
        },
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
        // The stamps go on counting, so a later join never reuses one.
        NetworkEvent::Gone => Network {
            join: network.join.next(Step::Land),
            ..Network::default()
        },
    }
}

#[cfg(test)]
mod tests {
    use super::{Connectivity, Link, Network, NetworkEvent, reduce_network};
    use crate::attempt::{Attempt, Attempts, OpStamp};
    use crate::units::Percent;

    fn wifi(strength: u16) -> Link {
        Link::Wifi {
            ssid: "home".to_owned(),
            strength: Percent(strength),
        }
    }

    fn net(link: Link, connectivity: Connectivity) -> Network {
        Network {
            link,
            connectivity,
            join: Attempts::default(),
        }
    }

    fn joining(link: Link, now: Attempt, minted: u32) -> Network {
        Network {
            join: Attempts {
                now,
                minted: OpStamp(minted),
            },
            ..net(link, Connectivity::Unknown)
        }
    }

    #[test]
    fn each_join_is_stamped_once_and_each_failure_anew() {
        use Attempt::{Failed, Quiet, Running};
        let s = OpStamp;
        // (name, before, link event, after)
        let cases = [
            (
                "entering connecting mints",
                joining(Link::Disconnected, Quiet, 0),
                Link::Connecting,
                joining(Link::Connecting, Running(s(1)), 1),
            ),
            (
                "the same join reported again keeps its stamp",
                joining(Link::Connecting, Running(s(1)), 1),
                Link::Connecting,
                joining(Link::Connecting, Running(s(1)), 1),
            ),
            (
                "a failed activation mints a new stamp",
                joining(Link::Connecting, Running(s(1)), 1),
                Link::Disconnected,
                joining(Link::Disconnected, Failed(s(2)), 2),
            ),
            (
                "the failure holds while disconnected",
                joining(Link::Disconnected, Failed(s(2)), 2),
                Link::Disconnected,
                joining(Link::Disconnected, Failed(s(2)), 2),
            ),
            (
                "a retry is a new join",
                joining(Link::Disconnected, Failed(s(2)), 2),
                Link::Connecting,
                joining(Link::Connecting, Running(s(3)), 3),
            ),
            (
                "joining lands",
                joining(Link::Connecting, Running(s(3)), 3),
                wifi(60),
                joining(wifi(60), Quiet, 3),
            ),
            (
                "a wire lands a failure too",
                joining(Link::Disconnected, Failed(s(2)), 2),
                Link::Wired,
                joining(Link::Wired, Quiet, 2),
            ),
            (
                "dropping an up link is no failure",
                joining(wifi(60), Quiet, 3),
                Link::Disconnected,
                joining(Link::Disconnected, Quiet, 3),
            ),
        ];
        for (name, before, link, want) in cases {
            assert_eq!(
                reduce_network(before, NetworkEvent::Link(link)),
                want,
                "{name}"
            );
        }
        assert_eq!(
            reduce_network(
                joining(Link::Connecting, Running(s(5)), 5),
                NetworkEvent::Gone
            ),
            joining(Link::Disconnected, Quiet, 5),
            "gone keeps the count"
        );
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
