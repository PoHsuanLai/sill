//! What kind of session sill runs in, and what the locker service does there: sill's own lock
//! screen, whose logind triggers depend on the session ([`Host`]).

use super::machine::{Runner, Triggers};

/// What kind of session sill is running in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Host {
    /// cosmic-session is running for this user: its own components answer logind.
    CosmicSession,
    /// sill's own session (`dist/sill-session`: `XDG_CURRENT_DESKTOP` names COSMIC, no
    /// cosmic-session).
    SillSession,
    /// Another desktop's session, e.g. sill on a nested compositor during development:
    /// logind's signals belong to that desktop.
    Guest,
}

impl Host {
    /// From whether cosmic-session runs and `XDG_CURRENT_DESKTOP` (colon-separated).
    pub fn of(cosmic_session: SessionManager, current_desktop: &str) -> Host {
        match cosmic_session {
            SessionManager::CosmicSession => Host::CosmicSession,
            SessionManager::Other
                if current_desktop
                    .split(':')
                    .any(|d| d.eq_ignore_ascii_case("COSMIC")) =>
            {
                Host::SillSession
            }
            SessionManager::Other => Host::Guest,
        }
    }

    fn triggers(self) -> Triggers {
        match self {
            Host::SillSession => Triggers::FollowLogind,
            Host::CosmicSession | Host::Guest => Triggers::IgnoreLogind,
        }
    }
}

/// Whether a cosmic-session process runs for this user.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SessionManager {
    CosmicSession,
    Other,
}

/// What the service does, resolved.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum LockerMode {
    /// Never lock (the service before it is configured, and tests).
    Off,
    /// sill's own lock screen: the daemon holds shell-host's session lock while the service's
    /// phase is `Locking` or `Locked`.
    Own { triggers: Triggers },
}

impl LockerMode {
    /// sill's own lock screen, with the triggers `host` gives it.
    pub fn for_host(host: Host) -> LockerMode {
        LockerMode::Own {
            triggers: host.triggers(),
        }
    }

    /// The machine's view of it.
    pub fn runner(&self) -> (Runner, Triggers) {
        match self {
            LockerMode::Off => (Runner::Off, Triggers::IgnoreLogind),
            LockerMode::Own { triggers } => (Runner::Own, *triggers),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Host, LockerMode, SessionManager};
    use crate::locker::machine::Triggers;

    #[test]
    fn only_a_sill_session_follows_logind() {
        let cases = [
            (SessionManager::CosmicSession, "COSMIC", Host::CosmicSession),
            (SessionManager::Other, "COSMIC", Host::SillSession),
            (SessionManager::Other, "sill:COSMIC", Host::SillSession),
            (SessionManager::Other, "KDE", Host::Guest),
            (SessionManager::Other, "", Host::Guest),
        ];
        for (manager, desktop, want) in cases {
            assert_eq!(Host::of(manager, desktop), want, "{manager:?} {desktop:?}");
        }
        assert_eq!(
            LockerMode::for_host(Host::SillSession),
            LockerMode::Own {
                triggers: Triggers::FollowLogind
            }
        );
        assert_eq!(
            LockerMode::for_host(Host::CosmicSession),
            LockerMode::Own {
                triggers: Triggers::IgnoreLogind
            }
        );
    }
}
