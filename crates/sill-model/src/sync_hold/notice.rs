//! Telling the person: the words, the one action, and finding the notification already shown.

use crate::notifications::{ActionKey, NotesState, NotificationId};

use super::model::{Dataset, Hold};

/// The desktop entry the notifications carry; it names them among the notifications held.
pub const NOTICE_ENTRY: &str = "sill-sync-hold";

/// What an action key says before the dataset it is for.
const KEY_PREFIX: &str = "open-settings:";

/// A notification's words and the label of its one button.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Notice {
    pub summary: String,
    pub body: String,
    pub button: String,
}

/// What the person is told about `hold`. Settings, not the shell, offers the choice.
pub fn notice_text(hold: &Hold) -> Notice {
    let removed = match hold.discard == hold.held {
        true => format!("All {} files were removed online.", hold.held),
        false => format!(
            "{} of {} files were removed online.",
            hold.discard, hold.held
        ),
    };
    Notice {
        summary: format!("Files removed online in {}", hold.account_name),
        body: format!("{removed} Open Settings to decide."),
        button: "Open Settings".to_owned(),
    }
}

/// The action key of the button that opens Settings for `dataset`.
pub fn open_key(dataset: &Dataset) -> ActionKey {
    ActionKey(format!("{KEY_PREFIX}{}", dataset.as_str()))
}

/// The dataset a button's action key opens Settings for; `None` for any other key.
pub fn dataset_of(key: &ActionKey) -> Option<Dataset> {
    key.0
        .strip_prefix(KEY_PREFIX)
        .filter(|dataset| !dataset.is_empty())
        .map(Dataset::new)
}

/// The notification for `dataset` already held, banner or history.
pub fn shown_for(notes: &NotesState, dataset: &Dataset) -> Option<NotificationId> {
    let key = open_key(dataset);
    let banners = notes.live.banners.iter().map(|banner| &banner.note);
    let kept = notes.history.groups.iter().flat_map(|group| &group.notes);
    banners
        .chain(kept)
        .filter(|note| note.app.desktop_entry.as_deref() == Some(NOTICE_ENTRY))
        .find(|note| note.actions.iter().any(|action| action.key == key))
        .map(|note| note.id)
}
