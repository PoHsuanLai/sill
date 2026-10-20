//! The session's state: which power actions logind offers right now.

use serde::{Deserialize, Serialize};
pub use sill_launcher::SessionAction;

use crate::units::Presence;

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
        match word {
            "yes" => Offer::Available,
            "challenge" => Offer::NeedsAuth,
            _ => Offer::Unavailable,
        }
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
    /// Every action offered (a dry run's session).
    pub fn everything() -> Session {
        Session {
            presence: Presence::Present,
            lock: Offer::Available,
            log_out: Offer::Available,
            restart: Offer::Available,
            power_off: Offer::Available,
            suspend: Offer::Available,
        }
    }

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
