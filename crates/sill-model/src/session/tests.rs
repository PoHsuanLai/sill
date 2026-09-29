use sill_launcher::SystemCommand;

use super::{Offer, Session, SessionAction, SessionEvent, step};
use crate::units::Presence;

#[test]
fn launcher_rows_name_their_session_actions() {
    const CASES: &[(SystemCommand, Option<SessionAction>)] = &[
        (SystemCommand::Lock, Some(SessionAction::Lock)),
        (SystemCommand::LogOut, Some(SessionAction::LogOut)),
        (SystemCommand::Suspend, Some(SessionAction::Suspend)),
        (SystemCommand::Restart, Some(SessionAction::Restart)),
        (SystemCommand::ShutDown, Some(SessionAction::PowerOff)),
        (SystemCommand::Reload, None),
    ];
    for &(command, want) in CASES {
        assert_eq!(SessionAction::of_system(command), want, "{command:?}");
    }
}

#[test]
fn logind_words_map_to_offers() {
    const CASES: &[(&str, Offer)] = &[
        ("yes", Offer::Available),
        ("challenge", Offer::NeedsAuth),
        ("no", Offer::Unavailable),
        ("na", Offer::Unavailable),
        ("yes ", Offer::Unavailable),
        ("", Offer::Unavailable),
    ];
    for &(word, want) in CASES {
        assert_eq!(Offer::from_logind(word), want, "{word:?}");
    }
}

#[test]
fn a_probe_replaces_and_gone_clears() {
    let probed = Session {
        presence: Presence::Present,
        lock: Offer::Available,
        log_out: Offer::Available,
        restart: Offer::NeedsAuth,
        power_off: Offer::Available,
        suspend: Offer::Unavailable,
    };
    let cases = [
        (Session::default(), SessionEvent::Probed(probed), probed),
        (probed, SessionEvent::Gone, Session::default()),
    ];
    for (before, event, want) in cases {
        assert_eq!(step(before, event), want, "{event:?}");
    }
    assert_eq!(probed.offer(SessionAction::Restart), Offer::NeedsAuth);
}
