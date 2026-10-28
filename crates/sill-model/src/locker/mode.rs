//! What kind of session sill runs in, and what the locker service does there: sill's own lock
//! screen, whose logind triggers depend on the session ([`Host`]).

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

    /// Whether logind's `Lock` and `PrepareForSleep` are ours to act on: a sill session only.
    pub fn follows_logind(self) -> bool {
        self == Host::SillSession
    }
}

/// The desktop names a sill session carries in `XDG_CURRENT_DESKTOP`.
const OWN_DESKTOPS: [&str; 1] = ["quire"];

fn is_own_desktop(name: &str) -> bool {
    OWN_DESKTOPS
        .iter()
        .any(|own| own.eq_ignore_ascii_case(name))
}

#[cfg(test)]
mod tests {
    use super::Host;

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
        assert!(Host::SillSession.follows_logind());
        assert!(!Host::Guest.follows_logind());
    }
}
