//! The check a signal from a shell-bound sender makes: [`admit`](super::admit)'s, but a refused
//! sender is said once. Any bus client may emit a signal on a path the shell listens to, and a
//! signal has no one to answer, so a refusal per signal would be a procfs check and a journal
//! line per signal. A unique bus name is never handed out twice, so a sender refused once is
//! refused for as long as that name lives and is skipped unchecked after the first time.

use std::collections::VecDeque;

use peephole_bus::peephole_core::Refused;
use zbus::message::Header;
use zbus::names::OwnedUniqueName;

use super::Peers;
use super::claim::{Claim, check, log_refusal};

/// How many refused names are remembered. A name that falls out of the memory (the oldest goes
/// first) is checked, and said, once more if it keeps signalling.
const REFUSED_NAMES_MAX: usize = 256;

/// The unique names of the senders already refused, oldest first, and no more than
/// [`REFUSED_NAMES_MAX`] of them.
#[derive(Debug, Default)]
struct RefusedNames(VecDeque<String>);

impl RefusedNames {
    /// Whether `name` was refused already.
    fn holds(&self, name: &str) -> bool {
        self.0.iter().any(|known| known == name)
    }

    /// Remember `name` as refused; the oldest name is forgotten when the memory is full.
    fn remember(&mut self, name: &str) {
        if self.holds(name) {
            return;
        }
        if self.0.len() >= REFUSED_NAMES_MAX {
            self.0.pop_front();
        }
        self.0.push_back(name.to_owned());
    }
}

/// The senders of the signals one loop receives, and which of them were refused.
#[derive(Debug, Default)]
pub(crate) struct SignalSenders {
    refused: RefusedNames,
}

impl SignalSenders {
    /// The sender of the signal `header` belongs to and the passing claim's tag, when it holds
    /// the role of one of `claims` and owns that claim's name; `None` for anyone else. The first
    /// refusal of a sender is logged and every later signal of that sender is dropped without
    /// asking the system who it is.
    pub(crate) async fn admit<T: Copy>(
        &mut self,
        peers: &Peers,
        header: &Header<'_>,
        claims: &[Claim<T>],
    ) -> Option<(OwnedUniqueName, T)> {
        let sender = header.sender().map(zbus::names::UniqueName::as_str);
        if sender.is_some_and(|name| self.refused.holds(name)) {
            return None;
        }
        match check(peers, header, claims).await {
            Ok(admitted) => Some(admitted),
            Err(refused) => {
                self.refuse(sender, claims, &refused);
                None
            }
        }
    }

    /// Say that `sender` was refused and remember it, so that it is not said again.
    fn refuse<T>(&mut self, sender: Option<&str>, claims: &[Claim<T>], refused: &Refused) {
        log_refusal(sender, claims, refused);
        if let Some(name) = sender {
            self.refused.remember(name);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{REFUSED_NAMES_MAX, RefusedNames};

    #[test]
    fn a_name_is_held_once_however_often_it_is_remembered() {
        let mut names = RefusedNames::default();
        assert!(!names.holds(":1.7"));
        for _ in 0..1000 {
            names.remember(":1.7");
        }
        assert!(names.holds(":1.7"));
        assert!(!names.holds(":1.8"));
        assert_eq!(names.0.len(), 1);
    }

    #[test]
    fn the_memory_is_bounded_and_forgets_the_oldest_name_first() {
        let mut names = RefusedNames::default();
        let name = |n: usize| format!(":1.{n}");
        for n in 0..REFUSED_NAMES_MAX + 10 {
            names.remember(&name(n));
        }
        assert_eq!(names.0.len(), REFUSED_NAMES_MAX);
        assert!(!names.holds(&name(0)));
        assert!(!names.holds(&name(9)));
        assert!(names.holds(&name(10)));
        assert!(names.holds(&name(REFUSED_NAMES_MAX + 9)));
    }
}
