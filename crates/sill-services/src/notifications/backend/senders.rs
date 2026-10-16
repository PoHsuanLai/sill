//! Who each held notification belongs to, and how fast each sender may post: the live loop's
//! record of the callers on the bus. A notification may be replaced or closed over the bus only
//! by the connection that posted it; one the shell posted itself, or loaded from the history
//! file, belongs to no connection and so to none of them.

use std::collections::HashMap;

use sill_model::notifications::{
    Allowance, NotesState, NotificationId, SENDERS_TRACKED_MAX, UnixMs, holds,
};

use super::dbus::Sender;

/// The callers the loop knows.
#[derive(Debug, Default)]
pub(super) struct Senders {
    owners: HashMap<NotificationId, Sender>,
    allowances: HashMap<Sender, Allowance>,
}

/// Whether a sender may post now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Admission {
    Allowed,
    TooMany,
}

impl Senders {
    /// Take one notification from `sender`'s allowance at `now`.
    pub(super) fn admit(&mut self, sender: &Sender, now: UnixMs) -> Admission {
        let held = self
            .allowances
            .get(sender)
            .copied()
            .unwrap_or_else(|| Allowance::full(now));
        let (left, admission) = match held.take(now) {
            Ok(left) => (left, Admission::Allowed),
            Err(left) => (left, Admission::TooMany),
        };
        self.make_room_for(sender, now);
        self.allowances.insert(sender.clone(), left);
        admission
    }

    /// Forget the senders with their whole burst back, and when still more than
    /// [`SENDERS_TRACKED_MAX`] are tracked, the one counted longest ago, so the table cannot
    /// grow with the number of connections that have ever posted.
    fn make_room_for(&mut self, sender: &Sender, now: UnixMs) {
        if self.allowances.len() < SENDERS_TRACKED_MAX || self.allowances.contains_key(sender) {
            return;
        }
        self.allowances
            .retain(|_, allowance| !allowance.is_idle(now));
        if self.allowances.len() < SENDERS_TRACKED_MAX {
            return;
        }
        let oldest = self
            .allowances
            .iter()
            .min_by_key(|(_, allowance)| allowance.counted_at())
            .map(|(sender, _)| sender.clone());
        if let Some(oldest) = oldest {
            self.allowances.remove(&oldest);
        }
    }

    /// The id `sender` may replace: `replaces` when `sender` posted it, else none (a new
    /// notification, as for an id nobody holds).
    pub(super) fn replaceable(
        &self,
        sender: &Sender,
        replaces: Option<NotificationId>,
    ) -> Option<NotificationId> {
        replaces.filter(|id| self.owners.get(id) == Some(sender))
    }

    /// Whether `sender` posted the held notification `id`.
    pub(super) fn owns(&self, sender: &Sender, id: NotificationId) -> bool {
        self.owners.get(&id) == Some(sender)
    }

    /// `id` posted by `sender`; `None` for one the shell posted itself.
    pub(super) fn record(&mut self, id: NotificationId, sender: Option<Sender>) {
        match sender {
            Some(sender) => {
                self.owners.insert(id, sender);
            }
            None => {
                self.owners.remove(&id);
            }
        }
    }

    /// Forget the owners of what `state` no longer holds.
    pub(super) fn forget_gone(&mut self, state: &NotesState) {
        self.owners.retain(|id, _| holds(state, *id));
    }
}

#[cfg(test)]
mod tests {
    use sill_model::notifications::{
        NotesState, NotificationId, SENDER_BURST, SENDERS_TRACKED_MAX, UnixMs,
    };

    use super::{Admission, Senders};
    use crate::notifications::backend::dbus::Sender;

    fn sender(name: &str) -> Sender {
        Sender(name.to_owned())
    }

    #[test]
    fn a_sender_that_floods_is_refused_while_another_is_not() {
        let mut senders = Senders::default();
        let (noisy, calm) = (sender(":1.1"), sender(":1.2"));
        let at = UnixMs(10_000);
        for _ in 0..SENDER_BURST {
            assert_eq!(senders.admit(&noisy, at), Admission::Allowed);
        }
        assert_eq!(senders.admit(&noisy, at), Admission::TooMany);
        assert_eq!(senders.admit(&calm, at), Admission::Allowed);
    }

    #[test]
    fn the_table_of_senders_stays_bounded() {
        let mut senders = Senders::default();
        let at = UnixMs(10_000);
        for n in 0..(SENDERS_TRACKED_MAX * 3) {
            senders.admit(&sender(&format!(":1.{n}")), at);
        }
        assert!(senders.allowances.len() <= SENDERS_TRACKED_MAX);
    }

    #[test]
    fn only_the_poster_may_replace_or_close_and_the_shells_own_belong_to_no_one() {
        let mut senders = Senders::default();
        let (mine, other) = (sender(":1.1"), sender(":1.2"));
        let (posted, shells) = (NotificationId(1), NotificationId(2));
        senders.record(posted, Some(mine.clone()));
        senders.record(shells, None);
        assert_eq!(senders.replaceable(&mine, Some(posted)), Some(posted));
        assert_eq!(senders.replaceable(&other, Some(posted)), None);
        assert_eq!(senders.replaceable(&mine, Some(shells)), None);
        assert_eq!(senders.replaceable(&mine, None), None);
        assert!(senders.owns(&mine, posted));
        assert!(!senders.owns(&other, posted));
        assert!(!senders.owns(&mine, shells));
        senders.forget_gone(&NotesState::default());
        assert!(
            !senders.owns(&mine, posted),
            "forgotten with the notification"
        );
    }
}
