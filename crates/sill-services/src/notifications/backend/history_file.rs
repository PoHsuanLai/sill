//! `$XDG_STATE_HOME/sill/notifications.json`: the history, kept across restarts (effects: the
//! filesystem). Written whole and durably after every change that
//! touches the history (`NotificationsEffect::SaveHistory`); read once at start. A file that cannot
//! be read is kept as `.damaged` rather than written over.
//!
//! An image the sender sent as pixels (`image-data`) is not written: the file would grow by
//! megabytes for a contact photo, and an image path or icon name is kept as it was.

use std::path::{Path, PathBuf};

use sill_model::notifications::{STORED_VERSION, StoredHistory};

use crate::durable::{self, Access};
use crate::error::ServiceError;

/// The file's name inside `$XDG_STATE_HOME/sill`.
pub const FILE_NAME: &str = "notifications.json";

/// `notifications.json` in sill's folder of `dirs`' state directory.
pub fn path_in(dirs: &crate::env::Dirs) -> PathBuf {
    dirs.sill_state().join(FILE_NAME)
}

/// The file at `path`; empty when there is none, it cannot be read, or it is another version
/// (a lost history costs the list, never a failed start).
pub fn load(path: &Path) -> StoredHistory {
    let Ok(text) = std::fs::read_to_string(path) else {
        return StoredHistory::default();
    };
    match serde_json::from_str::<StoredHistory>(&text) {
        Ok(stored) if stored.version == STORED_VERSION => stored,
        // Not this version's, or not readable: the next save would replace it, so it is kept.
        Ok(_) | Err(_) => {
            crate::durable::set_aside(path);
            StoredHistory::default()
        }
    }
}

/// Write `stored` to `path` (its directory created) durably: private to the person, whole or not
/// at all ([`crate::durable`]).
pub fn save(path: &Path, stored: &StoredHistory) -> Result<(), ServiceError> {
    let text = serde_json::to_string(stored).map_err(|e| ServiceError::Io {
        path: path.to_path_buf(),
        source: std::io::Error::other(e),
    })?;
    durable::write_file(path, text.as_bytes(), Access::OwnerOnly)
}

#[cfg(test)]
mod tests {
    use super::{load, save};
    use sill_model::notifications::Body;
    use sill_model::notifications::NotesState;
    use sill_model::notifications::{AppGroup, History};
    use sill_model::notifications::{AppKey, NotificationId, UnixMs};
    use sill_model::notifications::{
        AppRef, Expiry, Hints, IconRef, NoteImage, Notification, Pixels,
    };
    use sill_model::notifications::{STORED_VERSION, StoredHistory, for_disk};

    fn note(id: u32, image: NoteImage) -> Notification {
        Notification {
            id: NotificationId(id),
            arrived: UnixMs(1_790_000_000_000),
            app: AppRef {
                name: "Mail".to_owned(),
                icon: IconRef::Name("mail-unread".to_owned()),
                desktop_entry: Some("org.example.Mail".to_owned()),
            },
            summary: "Ada".to_owned(),
            body: Body::plain("the engine"),
            actions: Vec::new(),
            hints: Hints {
                image,
                ..Hints::default()
            },
            expiry: Expiry::Default,
        }
    }

    fn state() -> NotesState {
        let pixels = NoteImage::Pixels(Pixels {
            width: 1,
            height: 1,
            rgba: vec![1, 2, 3, 255],
        });
        NotesState {
            history: History {
                groups: vec![AppGroup {
                    key: AppKey("org.example.Mail".to_owned()),
                    notes: vec![note(7, pixels), note(6, NoteImage::Icon(IconRef::None))],
                }],
            },
            last_id: NotificationId(7),
            ..NotesState::default()
        }
    }

    #[test]
    fn pixels_are_not_written_and_the_rest_round_trips() {
        let dir = std::env::temp_dir().join(format!("sill-notes-{}", std::process::id()));
        let path = dir.join("state/sill/notifications.json");
        let stored = for_disk(&state());
        assert_eq!(stored.version, STORED_VERSION);
        assert_eq!(stored.last_id, NotificationId(7));
        assert_eq!(
            stored.history.groups[0].notes[0].hints.image,
            NoteImage::None
        );
        assert_eq!(
            stored.history.groups[0].notes[1].hints.image,
            NoteImage::Icon(IconRef::None)
        );
        save(&path, &stored).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(load(&path), stored);
        assert_eq!(
            std::fs::read_dir(path.parent().unwrap_or(&dir))
                .map(|entries| entries.count())
                .unwrap_or_default(),
            1,
            "no temporary file is left beside the history"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_missing_broken_or_foreign_file_is_empty() {
        let dir = std::env::temp_dir().join(format!("sill-notes-bad-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(load(&dir.join("absent.json")), StoredHistory::default());
        let broken = dir.join("broken.json");
        std::fs::write(&broken, "{ not json").unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(load(&broken), StoredHistory::default());
        assert!(
            dir.join("broken.json.damaged").exists(),
            "the unreadable history is kept, not left for the next save to replace"
        );
        let foreign = dir.join("foreign.json");
        let other = StoredHistory {
            version: STORED_VERSION + 1,
            ..for_disk(&state())
        };
        std::fs::write(&foreign, serde_json::to_string(&other).unwrap_or_default())
            .unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(load(&foreign), StoredHistory::default());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
