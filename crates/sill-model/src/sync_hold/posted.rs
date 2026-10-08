//! The notifications sill posted: which id carries which dataset's button, so a press is answered
//! only when the id is one sill itself posted for that dataset.

use std::collections::HashMap;

use crate::notifications::{ActionKey, NotesState, NotificationId};

use super::model::Dataset;
use super::notice::{dataset_of, shown_for};

/// The id of the notification posted for each dataset, kept from when it was seen in the notes
/// until the server says it closed or the hold ends. It outlives the notes' own copy on purpose:
/// a pressed button leaves the notes before its `ActionInvoked` is heard.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Posted(HashMap<Dataset, NotificationId>);

impl Posted {
    /// Remember the notification the notes hold for each of `datasets`.
    pub fn learn<'a>(
        &mut self,
        notes: &NotesState,
        datasets: impl IntoIterator<Item = &'a Dataset>,
    ) {
        for dataset in datasets {
            if let Some(id) = shown_for(notes, dataset) {
                self.0.insert(dataset.clone(), id);
            }
        }
    }

    /// The server closed `id`: it answers nothing from now on.
    pub fn closed(&mut self, id: NotificationId) {
        self.0.retain(|_, kept| *kept != id);
    }

    /// The hold ended, or its notification is being replaced.
    pub fn forget(&mut self, dataset: &Dataset) {
        self.0.remove(dataset);
    }

    /// The dataset a press answers: the one whose key the button carried, and only when `id` is
    /// the notification posted for it.
    pub fn answers(&self, id: NotificationId, key: &ActionKey) -> Option<Dataset> {
        dataset_of(key).filter(|dataset| self.0.get(dataset) == Some(&id))
    }
}
