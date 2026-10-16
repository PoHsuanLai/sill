use super::{
    BusPeer, INHIBIT_TEXT_MAX_BYTES, INHIBITORS_PER_PEER_MAX, InhibitCookie, InhibitMsg,
    InhibitReply, Inhibition, Inhibitors, LogindIdle, inhibit_step, inhibition, logind_idle,
};

fn peer(name: &str) -> BusPeer {
    BusPeer(name.to_owned())
}

fn inhibit(name: &str) -> InhibitMsg {
    InhibitMsg::Inhibit {
        peer: peer(name),
        app: "mpv".to_owned(),
        reason: "playing".to_owned(),
    }
}

#[test]
fn cookies_count_up_and_only_their_peer_releases_them() {
    let (one, reply) = inhibit_step(Inhibitors::default(), inhibit(":1.1"));
    assert_eq!(reply, InhibitReply::Cookie(InhibitCookie(1)));
    let (two, reply) = inhibit_step(one, inhibit(":1.2"));
    assert_eq!(reply, InhibitReply::Cookie(InhibitCookie(2)));
    let (same, reply) = inhibit_step(
        two.clone(),
        InhibitMsg::UnInhibit {
            peer: peer(":1.2"),
            cookie: InhibitCookie(1),
        },
    );
    assert_eq!(reply, InhibitReply::Unknown, "another peer's cookie");
    assert_eq!(same, two);
    let (left, reply) = inhibit_step(
        same,
        InhibitMsg::UnInhibit {
            peer: peer(":1.1"),
            cookie: InhibitCookie(1),
        },
    );
    assert_eq!(reply, InhibitReply::Done);
    assert_eq!(left.held.len(), 1);
    let (none, reply) = inhibit_step(left, InhibitMsg::PeerGone(peer(":1.2")));
    assert_eq!(reply, InhibitReply::Done);
    assert!(none.held.is_empty());
}

#[test]
fn the_cookie_skips_zero_when_it_wraps() {
    let near = Inhibitors {
        held: Vec::new(),
        last: u32::MAX,
    };
    let (_, reply) = inhibit_step(near, inhibit(":1.9"));
    assert_eq!(reply, InhibitReply::Cookie(InhibitCookie(1)));
}

#[test]
fn logind_blocks_idle_when_its_list_names_idle() {
    const CASES: &[(&str, LogindIdle)] = &[
        ("", LogindIdle::Free),
        ("handle-power-key", LogindIdle::Free),
        ("handle-power-key:idle:sleep", LogindIdle::Blocked),
        ("idle", LogindIdle::Blocked),
        ("idleness", LogindIdle::Free),
    ];
    for &(text, want) in CASES {
        assert_eq!(logind_idle(text), want, "{text:?}");
    }
}

#[test]
fn either_kind_holds_idle_off() {
    let (held, _) = inhibit_step(Inhibitors::default(), inhibit(":1.1"));
    let cases = [
        (Inhibitors::default(), LogindIdle::Free, Inhibition::Free),
        (Inhibitors::default(), LogindIdle::Blocked, Inhibition::Held),
        (held.clone(), LogindIdle::Free, Inhibition::Held),
        (held, LogindIdle::Blocked, Inhibition::Held),
    ];
    for (inhibitors, logind, want) in cases {
        assert_eq!(inhibition(&inhibitors, logind), want, "{logind:?}");
    }
}

#[test]
fn a_peer_holds_a_few_inhibitors_and_is_refused_past_them() {
    let flood = (0..INHIBITORS_PER_PEER_MAX).fold(Inhibitors::default(), |held, _| {
        inhibit_step(held, inhibit(":1.1")).0
    });
    assert_eq!(flood.held.len(), INHIBITORS_PER_PEER_MAX);
    let (refused, reply) = inhibit_step(flood.clone(), inhibit(":1.1"));
    assert_eq!(reply, InhibitReply::TooMany);
    assert_eq!(refused, flood, "a refusal changes nothing");
    let (other, reply) = inhibit_step(flood, inhibit(":1.2"));
    assert!(
        matches!(reply, InhibitReply::Cookie(_)),
        "another peer is not held back"
    );
    assert_eq!(other.held.len(), INHIBITORS_PER_PEER_MAX + 1);
}

#[test]
fn what_a_peer_says_of_itself_is_cut() {
    let long = "x".repeat(10_000);
    let (held, _) = inhibit_step(
        Inhibitors::default(),
        InhibitMsg::Inhibit {
            peer: peer(":1.1"),
            app: long.clone(),
            reason: long,
        },
    );
    assert!(held.held[0].app.len() <= INHIBIT_TEXT_MAX_BYTES + 3);
    assert!(held.held[0].reason.len() <= INHIBIT_TEXT_MAX_BYTES + 3);
}
