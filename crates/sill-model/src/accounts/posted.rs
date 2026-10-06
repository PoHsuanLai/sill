//! The notices sill posted: which notification id carries which account's sign-in button, so a
//! press is answered only when the id is one sill itself posted for that account.

use std::collections::HashMap;

use crate::notifications::{ActionKey, NotesState, NotificationId};

use super::model::AccountKey;
use super::notice::{account_of, shown_for};

/// The id of the notice posted for each account, kept from when it was seen in the notes until
/// the server says it closed or the account is well again. It outlives the notes' own copy on
/// purpose: a pressed button leaves the notes before its `ActionInvoked` is heard.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Posted(HashMap<AccountKey, NotificationId>);

impl Posted {
    /// Remember the notice the notes hold for each of `accounts`; one posted afresh replaces the
    /// id kept for its account.
    pub fn learn<'a>(
        &mut self,
        notes: &NotesState,
        accounts: impl IntoIterator<Item = &'a AccountKey>,
    ) {
        for account in accounts {
            if let Some(id) = shown_for(notes, account) {
                self.0.insert(account.clone(), id);
            }
        }
    }

    /// The server closed `id`: it answers nothing from now on.
    pub fn closed(&mut self, id: NotificationId) {
        self.0.retain(|_, kept| *kept != id);
    }

    /// The account is well again, or its notice is being replaced: its id is no longer one.
    pub fn forget(&mut self, account: &AccountKey) {
        self.0.remove(account);
    }

    /// The account a press answers: the one whose key the button carried, and only when `id` is
    /// the notice posted for that account. Any other id, or the right one with another
    /// account's key, answers nothing.
    pub fn answers(&self, id: NotificationId, key: &ActionKey) -> Option<AccountKey> {
        account_of(key).filter(|account| self.0.get(account) == Some(&id))
    }
}
