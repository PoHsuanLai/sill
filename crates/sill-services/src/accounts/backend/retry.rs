//! Meeting accountd again after it refused the first call. accountd answers `AccessDenied` while
//! it does not yet know the caller's process, which a moment later it does; so a refused meeting
//! is tried again after a short, growing wait, and then only when accountd's name changes hands.

use std::time::Duration;

/// How long to wait before each further try, in order.
const BACKOFF: [Duration; 4] = [
    Duration::from_millis(100),
    Duration::from_millis(250),
    Duration::from_millis(500),
    Duration::from_secs(1),
];

/// Where the tries stand: how many waits have been spent since accountd's name last changed.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct Retry {
    spent: usize,
}

/// What to do after a refusal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Wait {
    /// Try again after this long (or sooner when the name changes).
    Then(Duration),
    /// The waits are spent: try again only when the name changes.
    OnNameChange,
}

impl Retry {
    /// The next wait after a refusal, and the state after spending it.
    pub(crate) fn refused(self) -> (Wait, Retry) {
        match BACKOFF.get(self.spent) {
            Some(wait) => (
                Wait::Then(*wait),
                Retry {
                    spent: self.spent + 1,
                },
            ),
            None => (Wait::OnNameChange, self),
        }
    }
}

/// Whether accountd's refusal is the one that passes: it does not know the caller yet.
pub(crate) fn is_refusal(error: &zbus::Error) -> bool {
    const DENIED: &str = "org.freedesktop.DBus.Error.AccessDenied";
    match error {
        zbus::Error::MethodError(name, ..) => name.as_str() == DENIED,
        zbus::Error::FDO(error) => matches!(**error, zbus::fdo::Error::AccessDenied(_)),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn waits_grow_then_only_the_name_decides() {
        let mut retry = Retry::default();
        let mut waits = Vec::new();
        for _ in 0..6 {
            let (wait, next) = retry.refused();
            waits.push(wait);
            retry = next;
        }
        assert_eq!(
            waits,
            [
                Wait::Then(Duration::from_millis(100)),
                Wait::Then(Duration::from_millis(250)),
                Wait::Then(Duration::from_millis(500)),
                Wait::Then(Duration::from_secs(1)),
                Wait::OnNameChange,
                Wait::OnNameChange,
            ]
        );
    }

    #[test]
    fn only_access_denied_is_a_refusal() {
        let denied = zbus::Error::FDO(Box::new(zbus::fdo::Error::AccessDenied("pid".into())));
        assert!(is_refusal(&denied));
        assert!(!is_refusal(&zbus::Error::InvalidReply));
    }
}
