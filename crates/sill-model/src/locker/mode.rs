//! What kind of session sill runs in, and what the locker service does there: sill's own lock
//! screen, whose logind triggers depend on the session ([`Host`]).

use super::{Runner, Triggers};

/// What kind of session sill is running in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Host {
    /// sill's own session (`dist/sill-session`, or the desktop's own: `XDG_CURRENT_DESKTOP`
    /// names quire).
    SillSession,
    /// Another desktop's session, e.g. sill on a nested compositor during development:
    /// logind's signals belong to that desktop.
    Guest,
}

impl Host {
    /// From `XDG_CURRENT_DESKTOP` (colon-separated).
    pub fn of(current_desktop: &str) -> Host {
        match current_desktop.split(':').any(is_own_desktop) {
            true => Host::SillSession,
            false => Host::Guest,
        }
    }

    fn triggers(self) -> Triggers {
        match self {
            Host::SillSession => Triggers::FollowLogind,
            Host::Guest => Triggers::IgnoreLogind,
        }
    }
}

/// The desktop names a sill session carries in `XDG_CURRENT_DESKTOP`.
const OWN_DESKTOPS: [&str; 1] = ["quire"];

fn is_own_desktop(name: &str) -> bool {
    OWN_DESKTOPS
        .iter()
        .any(|own| own.eq_ignore_ascii_case(name))
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
    use super::{Host, LockerMode};
    use crate::locker::Triggers;

    #[test]
    fn only_a_sill_session_follows_logind() {
        let cases = [
            ("quire", Host::SillSession),
            ("Quire", Host::SillSession),
            ("GNOME:quire", Host::SillSession),
            ("quirey", Host::Guest),
            ("KDE", Host::Guest),
            ("", Host::Guest),
        ];
        for (desktop, want) in cases {
            assert_eq!(Host::of(desktop), want, "{desktop:?}");
        }
        assert_eq!(
            LockerMode::for_host(Host::SillSession),
            LockerMode::Own {
                triggers: Triggers::FollowLogind
            }
        );
        assert_eq!(
            LockerMode::for_host(Host::Guest),
            LockerMode::Own {
                triggers: Triggers::IgnoreLogind
            }
        );
    }
}
