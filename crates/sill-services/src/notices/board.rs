//! One service's notices on the notifications service: post, replace, withdraw, and know which
//! notification ids are its own.

use std::hash::Hash;

use sill_model::notifications::{
    NotesState, Notices, NotificationId, NotificationsCommand, Posting,
};
use tokio::sync::watch;
use tokio::sync::watch::error::RecvError;

use crate::control::Control;

/// The notices of one service, one per key (an account, a dataset, a computer; `()` for a service
/// with a single notice).
#[derive(Debug)]
pub(crate) struct Board<K: Eq + Hash> {
    notifications: Control<NotificationsCommand>,
    notes: watch::Receiver<NotesState>,
    posted: Notices<K>,
    /// The notification already held for a key, banner or history.
    shown_for: fn(&NotesState, &K) -> Option<NotificationId>,
}

impl<K: Eq + Hash + Clone> Board<K> {
    /// A board over the notifications service, finding a key's held notice with `shown_for`.
    pub(crate) fn new(
        notifications: Control<NotificationsCommand>,
        notes: watch::Receiver<NotesState>,
        shown_for: fn(&NotesState, &K) -> Option<NotificationId>,
    ) -> Board<K> {
        Board {
            notifications,
            notes,
            posted: Notices::default(),
            shown_for,
        }
    }

    /// Resolves when the notifications service's state changed; an error when it is gone.
    pub(crate) async fn changed(&mut self) -> Result<(), RecvError> {
        self.notes.changed().await
    }

    /// Keep the id of each notice the notes now hold for one of `keys`.
    pub(crate) fn learn<'a>(&mut self, keys: impl IntoIterator<Item = &'a K>)
    where
        K: 'a,
    {
        self.posted
            .learn(&self.notes.borrow(), keys, self.shown_for);
    }

    /// Post the notice for `key`, in place of the one held when there is one. `posting` is given
    /// the id to replace.
    pub(crate) fn post(
        &mut self,
        key: &K,
        posting: impl FnOnce(Option<NotificationId>) -> Posting,
    ) {
        self.posted.forget(key);
        let replaces = (self.shown_for)(&self.notes.borrow(), key);
        self.send(NotificationsCommand::Post(Box::new(posting(replaces))));
    }

    /// Take the notice for `key` back, when one is held.
    pub(crate) fn withdraw(&mut self, key: &K) {
        self.posted.forget(key);
        if let Some(id) = (self.shown_for)(&self.notes.borrow(), key) {
            self.send(NotificationsCommand::Dismiss(id));
        }
    }

    /// The server closed `id`: it answers nothing from now on.
    pub(crate) fn closed(&mut self, id: NotificationId) {
        self.posted.closed(id);
    }

    /// What a press answers (see [`Notices::answers`]).
    pub(crate) fn answers<R>(
        &self,
        id: NotificationId,
        pressed: Option<R>,
        key_of: impl Fn(&R) -> &K,
    ) -> Option<R> {
        self.posted.answers(id, pressed, key_of)
    }

    /// The notifications service itself, for a notice that is not tied to a key.
    pub(crate) fn notifications(&self) -> Control<NotificationsCommand> {
        self.notifications.clone()
    }

    fn send(&self, command: NotificationsCommand) {
        // A stopped notifications service has nobody to tell.
        let _ = self.notifications.send(command);
    }
}
