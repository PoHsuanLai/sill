//! Who each held notification belongs to, and how fast each sender may post: the live loop's
//! record of the callers on the bus. A notification may be closed over the bus only by the
//! connection that posted it, and replaced by that connection or by a later one that names the
//! same app (so `notify-send -r` from a script keeps working across runs). One the shell posted
//! itself, or loaded from the history file, belongs to no connection and so to none of them.

use std::collections::HashMap;

use sill_model::notifications::{
    Allowance, AppKey, NotesState, NotificationId, SENDERS_TRACKED_MAX, UnixMs, holds,
};

use super::dbus::Sender;

/// The callers the loop knows.
#[derive(Debug, Default)]
pub(super) struct Senders {
    owners: HashMap<NotificationId, Owner>,
    allowances: HashMap<Sender, Allowance>,
}

/// The connection that posted a held notification, and the app it named.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Owner {
    pub(super) sender: Sender,
    pub(super) app: AppKey,
}

impl Owner {
    /// Whether a posting from `sender` naming `app` may replace this one: the same connection,
    /// or a later one that names the same (non-empty) app.
    fn yields_to(&self, sender: &Sender, app: &AppKey) -> bool {
        self.sender == *sender || (!app.0.is_empty() && self.app == *app)
    }
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

    /// The id a posting from `sender` naming `app` may replace: `replaces` when that
    /// connection posted it, or when it was posted for the same app; else none (a new
    /// notification, as for an id nobody holds).
    pub(super) fn replaceable(
        &self,
        sender: &Sender,
        app: &AppKey,
        replaces: Option<NotificationId>,
    ) -> Option<NotificationId> {
        replaces.filter(|id| {
            self.owners
                .get(id)
                .is_some_and(|owner| owner.yields_to(sender, app))
        })
    }

    /// Whether `sender` posted the held notification `id`.
    pub(super) fn owns(&self, sender: &Sender, id: NotificationId) -> bool {
        self.owners
            .get(&id)
            .is_some_and(|owner| owner.sender == *sender)
    }

    /// `id` posted by `owner`; `None` for one the shell posted itself.
    pub(super) fn record(&mut self, id: NotificationId, owner: Option<Owner>) {
        match owner {
            Some(owner) => {
                self.owners.insert(id, owner);
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
        AppKey, NotesState, NotificationId, SENDER_BURST, SENDERS_TRACKED_MAX, UnixMs,
    };

    use super::{Admission, Owner, Senders};
    use crate::notifications::backend::dbus::Sender;

    fn sender(name: &str) -> Sender {
        Sender(name.to_owned())
    }

    fn app(name: &str) -> AppKey {
        AppKey(name.to_owned())
    }

    fn owner(sender: &Sender, app: &AppKey) -> Option<Owner> {
        Some(Owner {
            sender: sender.clone(),
            app: app.clone(),
        })
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
    fn only_the_poster_may_close_the_same_app_may_replace_and_the_shells_own_belong_to_no_one() {
        let mut senders = Senders::default();
        let (mine, other) = (sender(":1.1"), sender(":1.2"));
        let (volume, mail, nameless) = (app("volume"), app("mail"), app(""));
        let (posted, shells, unnamed) = (NotificationId(1), NotificationId(2), NotificationId(3));
        senders.record(posted, owner(&mine, &volume));
        senders.record(shells, None);
        senders.record(unnamed, owner(&mine, &nameless));
        assert_eq!(
            senders.replaceable(&mine, &mail, Some(posted)),
            Some(posted)
        );
        assert_eq!(
            senders.replaceable(&other, &volume, Some(posted)),
            Some(posted),
            "a later run of the same script replaces its own notice"
        );
        assert_eq!(senders.replaceable(&other, &mail, Some(posted)), None);
        assert_eq!(senders.replaceable(&other, &nameless, Some(unnamed)), None);
        assert_eq!(senders.replaceable(&mine, &volume, Some(shells)), None);
        assert_eq!(
            senders.replaceable(&other, &app("sill-accounts"), Some(shells)),
            None
        );
        assert_eq!(senders.replaceable(&mine, &volume, None), None);
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
