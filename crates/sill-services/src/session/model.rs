//! The session's state: which power actions logind offers right now, and the one reducer.

use serde::{Deserialize, Serialize};
use sill_launcher::SystemCommand;

use crate::units::Presence;

/// One power action, in the power menu's order (design/20 §1.8: Log Out, Restart, Shut Down,
/// Suspend; Lock is the launcher's and the bar's, not a power-menu button).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionAction {
    Lock,
    LogOut,
    Restart,
    PowerOff,
    Suspend,
}

impl SessionAction {
    /// The power menu's buttons, left to right.
    pub const POWER_MENU: [SessionAction; 4] = [
        SessionAction::LogOut,
        SessionAction::Restart,
        SessionAction::PowerOff,
        SessionAction::Suspend,
    ];

    /// The session action a launcher system row names; `None` for `Reload`, which is the
    /// daemon's, not logind's.
    pub fn of_system(command: SystemCommand) -> Option<SessionAction> {
        match command {
            SystemCommand::Lock => Some(SessionAction::Lock),
            SystemCommand::LogOut => Some(SessionAction::LogOut),
            SystemCommand::Suspend => Some(SessionAction::Suspend),
            SystemCommand::Restart => Some(SessionAction::Restart),
            SystemCommand::ShutDown => Some(SessionAction::PowerOff),
            SystemCommand::Reload => None,
        }
    }
}

/// Whether logind will carry out an action now: its `Can*` answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Offer {
    /// `yes`.
    Available,
    /// `challenge`: polkit will ask for a password first.
    NeedsAuth,
    /// `no`, `na`, anything else, or logind never answered.
    #[default]
    Unavailable,
}

impl Offer {
    /// logind's `Can*` word (`yes`, `challenge`, `no`, `na`), compared whole.
    pub fn from_logind(word: &str) -> Offer {
        let _ = word;
        todo!("M5 fill: svc-session-bt (a)")
    }
}

/// What logind allows, as the power menu draws it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Session {
    /// Whether logind answered at all.
    pub presence: Presence,
    /// Lock and Log Out need no `Can*`: `Available` once this session's object is known.
    pub lock: Offer,
    pub log_out: Offer,
    /// `CanReboot`.
    pub restart: Offer,
    /// `CanPowerOff`.
    pub power_off: Offer,
    /// `CanSuspend`.
    pub suspend: Offer,
}

impl Session {
    /// What logind says about `action`.
    pub fn offer(&self, action: SessionAction) -> Offer {
        match action {
            SessionAction::Lock => self.lock,
            SessionAction::LogOut => self.log_out,
            SessionAction::Restart => self.restart,
            SessionAction::PowerOff => self.power_off,
            SessionAction::Suspend => self.suspend,
        }
    }
}

/// One thing the service learnt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum SessionEvent {
    /// A fresh probe: every offer at once.
    Probed(Session),
    /// logind is unreachable.
    Gone,
}

/// `session` after `event`.
pub fn reduce_session(session: Session, event: SessionEvent) -> Session {
    let _ = (session, event);
    todo!("M5 fill: svc-session-bt (a)")
}

#[cfg(test)]
mod tests {
    use super::{Offer, Session, SessionAction, SessionEvent, reduce_session};
    use crate::units::Presence;
    use sill_launcher::SystemCommand;

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
    #[ignore = "M5 fill: svc-session-bt (a)"]
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
    #[ignore = "M5 fill: svc-session-bt (a)"]
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
            assert_eq!(reduce_session(before, event), want, "{event:?}");
        }
        assert_eq!(probed.offer(SessionAction::Restart), Offer::NeedsAuth);
    }
}
