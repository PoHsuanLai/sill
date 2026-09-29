//! The network state's pure transition.

use super::{Activating, Link, Network, NetworkEvent};
use crate::attempt::{Attempt, AttemptStep as Step};
use crate::units::Percent;

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

/// What an `Activating` report means for the join under way, independent of `Link` (whose own
/// transitions land or fail it when the primary connection itself changes): Wi-Fi activating
/// starts one, or is the same one reported again; it going idle while one was running ends it
/// without the connection having become primary, a failed attempt; idle otherwise holds.
fn activating_step(now: Attempt, state: Activating) -> Step {
    match (now, state) {
        (_, Activating::Joining) => Step::Start,
        (Attempt::Running(_), Activating::Idle) => Step::Fail,
        (Attempt::Quiet | Attempt::Failed(_), Activating::Idle) => Step::Hold,
    }
}

/// `network` after `event`.
pub fn step(network: Network, event: NetworkEvent) -> Network {
    match event {
        NetworkEvent::Link(link) => Network {
            join: network.join.next(join_step(&network.link, &link)),
            link,
            ..network
        },
        // `link` stays: an activation reported here has not touched the primary connection. If it
        // does, the `Link` event that follows lands or holds it there.
        NetworkEvent::Activating(state) => Network {
            join: network.join.next(activating_step(network.join.now, state)),
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
