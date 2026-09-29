use super::{Activating, Connectivity, Link, Network, NetworkEvent, step};
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
        assert_eq!(step(before, NetworkEvent::Link(link)), want, "{name}");
    }
    assert_eq!(
        step(
            joining(Link::Connecting, Running(s(5)), 5),
            NetworkEvent::Gone
        ),
        joining(Link::Disconnected, Quiet, 5),
        "gone keeps the count"
    );
}

#[test]
fn activating_connection_steps_the_join_without_touching_link() {
    use Activating::{Idle, Joining};
    use Attempt::{Failed, Quiet, Running};
    let s = OpStamp;
    // (name, before, activating event, after)
    let cases = [
        (
            "idle to activating mints, the link stays disconnected",
            joining(Link::Disconnected, Quiet, 0),
            Joining,
            joining(Link::Disconnected, Running(s(1)), 1),
        ),
        (
            "activating to deactivated (idle) without landing fails",
            joining(Link::Disconnected, Running(s(1)), 1),
            Idle,
            joining(Link::Disconnected, Failed(s(2)), 2),
        ),
        (
            "an idle report with nothing running holds",
            joining(Link::Disconnected, Quiet, 0),
            Idle,
            joining(Link::Disconnected, Quiet, 0),
        ),
        (
            "an idle report after a failure holds, the failure stands",
            joining(Link::Disconnected, Failed(s(2)), 2),
            Idle,
            joining(Link::Disconnected, Failed(s(2)), 2),
        ),
        (
            "activating while a wire is primary: the wire stays, the join shows",
            net(Link::Wired, Connectivity::Full),
            Joining,
            Network {
                join: Attempts {
                    now: Running(s(1)),
                    minted: s(1),
                },
                ..net(Link::Wired, Connectivity::Full)
            },
        ),
        (
            "activating while a joined wifi is primary: it stays, the join shows",
            net(wifi(70), Connectivity::Full),
            Joining,
            Network {
                join: Attempts {
                    now: Running(s(1)),
                    minted: s(1),
                },
                ..net(wifi(70), Connectivity::Full)
            },
        ),
        (
            "a retry (a second attempt in a row) mints a distinct stamp",
            joining(Link::Wired, Failed(s(1)), 1),
            Joining,
            joining(Link::Wired, Running(s(2)), 2),
        ),
    ];
    for (name, before, state, want) in cases {
        assert_eq!(
            step(before, NetworkEvent::Activating(state)),
            want,
            "{name}"
        );
    }
}

#[test]
fn a_join_seen_only_through_activating_connection_still_lands() {
    // idle -> activating (ActivatingConnection, State 1) -> activated (PrimaryConnection
    // catches up once NetworkManager flips it over): the same stamp throughout.
    let idle = Network::default();
    let activating = step(idle, NetworkEvent::Activating(Activating::Joining));
    assert_eq!(
        activating,
        joining(Link::Disconnected, Attempt::Running(OpStamp(1)), 1),
        "activating"
    );
    let activated = step(activating, NetworkEvent::Link(wifi(80)));
    assert_eq!(
        activated,
        joining(wifi(80), Attempt::Quiet, 1),
        "activated, same stamp"
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
        assert_eq!(step(before, event), want, "{name}");
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
