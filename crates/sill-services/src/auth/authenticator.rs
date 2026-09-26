//! The seam between the service and PAM: [`Authenticator`], implemented by [`super::PamHelper`]
//! (the real one) and [`FakeAuthenticator`] (every test).

use std::fmt;
use std::time::Duration;

use super::types::{Attempt, AuthFault, Outcome, Verdict};

/// Checks one attempt. Blocking: the service calls it on tokio's blocking pool, never on a
/// surface's thread.
pub trait Authenticator: fmt::Debug + Send + Sync {
    fn check(&self, attempt: &Attempt) -> Verdict;
}

/// What a [`FakeAuthenticator`] answers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FakeRule {
    /// Accept this exact text, refuse anything else.
    Accepts(String),
    /// Refuse everything.
    RefusesAll,
    /// Answer `Unavailable(fault)` to everything.
    Fails(AuthFault),
}

/// A stand-in for PAM: a rule, and how long each check takes (a real check takes about 2 s when
/// wrong: `pam_unix`'s fail delay).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FakeAuthenticator {
    pub rule: FakeRule,
    pub delay: Duration,
}

impl FakeAuthenticator {
    /// Accepts `secret` at once.
    pub fn accepting(secret: &str) -> FakeAuthenticator {
        FakeAuthenticator {
            rule: FakeRule::Accepts(secret.to_owned()),
            delay: Duration::ZERO,
        }
    }
}

impl Authenticator for FakeAuthenticator {
    fn check(&self, attempt: &Attempt) -> Verdict {
        std::thread::sleep(self.delay);
        Verdict::plain(match &self.rule {
            FakeRule::Accepts(text) if text == attempt.secret.expose() => Outcome::Accepted,
            FakeRule::Accepts(_) | FakeRule::RefusesAll => Outcome::Refused,
            FakeRule::Fails(fault) => Outcome::Unavailable(*fault),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{Authenticator, FakeAuthenticator, FakeRule};
    use crate::auth::types::{Attempt, AuthFault, Outcome, Secret, UserName};
    use std::time::Duration;

    #[test]
    fn the_fake_answers_by_its_rule() {
        let attempt = |text: &str| Attempt {
            user: UserName("ada".to_owned()),
            secret: Secret::new(text),
        };
        let fake = |rule| FakeAuthenticator {
            rule,
            delay: Duration::ZERO,
        };
        let cases = [
            (
                FakeAuthenticator::accepting("hunter2"),
                "hunter2",
                Outcome::Accepted,
            ),
            (
                FakeAuthenticator::accepting("hunter2"),
                "hunter3",
                Outcome::Refused,
            ),
            (fake(FakeRule::RefusesAll), "hunter2", Outcome::Refused),
            (
                fake(FakeRule::Fails(AuthFault::TimedOut)),
                "x",
                Outcome::Unavailable(AuthFault::TimedOut),
            ),
        ];
        for (authenticator, text, want) in cases {
            assert_eq!(authenticator.check(&attempt(text)).outcome, want, "{text}");
        }
    }

    #[test]
    fn a_secret_never_prints() {
        assert_eq!(format!("{:?}", Secret::new("hunter2")), "Secret(..)");
    }
}
