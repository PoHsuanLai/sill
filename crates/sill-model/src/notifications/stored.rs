//! What the notifications history file holds: the history and the last id, pixel images dropped.

use serde::{Deserialize, Serialize};

use super::content::NoteImage;
use super::ids::NotificationId;
use super::{History, NotesState};

/// The shape this build writes. A file of another version is read as empty.
pub const STORED_VERSION: u32 = 1;

/// What the file holds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoredHistory {
    pub version: u32,
    /// The last id handed out, so ids after a restart do not collide with kept ones.
    pub last_id: NotificationId,
    pub history: History,
}

impl Default for StoredHistory {
    fn default() -> Self {
        StoredHistory {
            version: STORED_VERSION,
            last_id: NotificationId::default(),
            history: History::default(),
        }
    }
}

/// What `state` writes: its history and last id, pixel images dropped.
pub fn for_disk(state: &NotesState) -> StoredHistory {
    let mut history = state.history.clone();
    for note in history.groups.iter_mut().flat_map(|g| g.notes.iter_mut()) {
        if let NoteImage::Pixels(_) = note.hints.image {
            note.hints.image = NoteImage::None;
        }
    }
    StoredHistory {
        version: STORED_VERSION,
        last_id: state.last_id,
        history,
    }
}
