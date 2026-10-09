//! The notifications sill posted: which id carries which computer's buttons, so a press is
//! answered only when the id is one sill itself posted for that computer.

use std::collections::HashMap;

use crate::notifications::{ActionKey, NotesState, NotificationId};

use super::model::{Computer, Verdict};
use super::notice::{choice_of, shown_for};

/// The id of the notification posted for each computer, kept from when it was seen in the notes
/// until the server says it closed or the question ends. It outlives the notes' own copy on
/// purpose: a pressed button leaves the notes before its `ActionInvoked` is heard.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Posted(HashMap<Computer, NotificationId>);

impl Posted {
    /// Remember the notification the notes hold for each of `computers`.
    pub fn learn<'a>(
        &mut self,
        notes: &NotesState,
        computers: impl IntoIterator<Item = &'a Computer>,
    ) {
        for computer in computers {
            if let Some(id) = shown_for(notes, computer) {
                self.0.insert(computer.clone(), id);
            }
        }
    }

    /// The server closed `id`: it answers nothing from now on.
    pub fn closed(&mut self, id: NotificationId) {
        self.0.retain(|_, kept| *kept != id);
    }

    /// The question ended, or its notification is being replaced.
    pub fn forget(&mut self, computer: &Computer) {
        self.0.remove(computer);
    }

    /// What a press answers: the computer whose key the button carried and the verdict it stands
    /// for, and only when `id` is the notification posted for that computer.
    pub fn answers(&self, id: NotificationId, key: &ActionKey) -> Option<(Computer, Verdict)> {
        choice_of(key).filter(|(computer, _)| self.0.get(computer) == Some(&id))
    }
}
