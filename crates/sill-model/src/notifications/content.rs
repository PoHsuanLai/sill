//! What a notification says: the typed form of one `Notify` call ([`Posting`]) and the held
//! notification it becomes once the server has given it an id and a time ([`Notification`]).
//!
//! The D-Bus boundary (`dbus.rs`) turns the wire's strings, integers and `a{sv}` hints into
//! these once; `sill debug notify` builds a [`Posting`] directly, without a bus.

use std::path::PathBuf;
use std::sync::Arc;

use ds_core::word::Word;
use serde::{Deserialize, Serialize};

use super::ids::{ActionKey, AppKey, NotificationId, UnixMs};
use super::markup::Body;

/// One `Notify` call, parsed: everything but the id and the time.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Posting {
    /// `replaces_id`: `None` for the spec's 0.
    pub replaces: Option<NotificationId>,
    pub app: AppRef,
    pub summary: String,
    pub body: Body,
    /// Every action in the sender's order, `default` included (the body's click, drawn as no
    /// button).
    pub actions: Vec<Action>,
    pub hints: Hints,
    pub expiry: Expiry,
}

/// A notification the server holds: a posting with its id and arrival.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Notification {
    pub id: NotificationId,
    pub arrived: UnixMs,
    pub app: AppRef,
    pub summary: String,
    pub body: Body,
    pub actions: Vec<Action>,
    pub hints: Hints,
    pub expiry: Expiry,
}

impl Notification {
    /// `posting` held as `id`, arrived at `arrived`.
    pub fn posted(id: NotificationId, arrived: UnixMs, posting: Posting) -> Notification {
        let Posting {
            replaces: _,
            app,
            summary,
            body,
            actions,
            hints,
            expiry,
        } = posting;
        Notification {
            id,
            arrived,
            app,
            summary,
            body,
            actions,
            hints,
            expiry,
        }
    }

    /// The actions drawn as buttons: every one but `default`.
    pub fn buttons(&self) -> impl Iterator<Item = &Action> {
        self.actions
            .iter()
            .filter(|action| !action.key.is_default())
    }

    /// Whether a click on the body invokes something: the sender offered `default`.
    pub fn default_action(&self) -> Option<&Action> {
        self.actions.iter().find(|action| action.key.is_default())
    }
}

/// Who sent it: the name to draw, an icon, and the desktop entry it claims.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AppRef {
    /// `app_name` as sent (may be empty).
    pub name: String,
    /// `app_icon`.
    pub icon: IconRef,
    /// The `desktop-entry` hint without `.desktop`, when sent.
    pub desktop_entry: Option<String>,
}

impl AppRef {
    /// The key it groups under: the desktop entry when there is one, else the name.
    pub fn key(&self) -> AppKey {
        match &self.desktop_entry {
            Some(entry) if !entry.is_empty() => AppKey(entry.clone()),
            _ => AppKey(self.name.clone()),
        }
    }
}

/// An icon named by the sender: `app_icon`, or the `image-path` hint.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum IconRef {
    /// Empty: the surface falls back to the desktop entry's icon, then a generic glyph.
    #[default]
    None,
    /// A freedesktop icon name (`mail-unread`).
    Name(String),
    /// A file, from a `file://` URI or an absolute path.
    File(PathBuf),
}

impl IconRef {
    /// `app_icon` or `image-path` as sent: empty is none, a `file://` URI or an absolute path
    /// is a file, anything else a theme name.
    pub fn from_wire(wire: &str) -> IconRef {
        match wire {
            "" => IconRef::None,
            uri if uri.starts_with("file://") => {
                IconRef::File(PathBuf::from(&uri["file://".len()..]))
            }
            path if path.starts_with('/') => IconRef::File(PathBuf::from(path)),
            name => IconRef::Name(name.to_owned()),
        }
    }
}

/// One action: its key and the label its button shows.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Action {
    pub key: ActionKey,
    pub label: String,
}

/// `Notify`'s `actions`, a flat list of key, label, key, label: paired in order. A trailing key
/// with no label is dropped.
pub fn pair_actions(flat: &[String]) -> Vec<Action> {
    flat.as_chunks::<2>()
        .0
        .iter()
        .map(|[key, label]| Action {
            key: ActionKey(key.clone()),
            label: label.clone(),
        })
        .collect()
}

/// The hints sill reads; the rest are ignored.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct Hints {
    /// `urgency` (byte).
    pub urgency: NotifyUrgency,
    /// `category` (`email.arrived`), when sent.
    pub category: Option<String>,
    /// `image-data` (or the deprecated `image_data`, `icon_data`), else `image-path`.
    pub image: NoteImage,
    /// `transient`.
    pub transience: Transience,
    /// `resident`.
    pub residency: Residency,
    /// `sound-name`, `sound-file`, `suppress-sound`.
    pub sound: SoundHint,
}

/// The `urgency` hint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, Word)]
#[serde(rename_all = "snake_case")]
pub enum NotifyUrgency {
    Low,
    #[default]
    Normal,
    /// Shows through Do Not Disturb, and persists until acted on.
    Critical,
}

impl NotifyUrgency {
    /// The hint's byte: 0 low, 2 critical, anything else normal.
    pub fn from_wire(byte: u8) -> NotifyUrgency {
        match byte {
            0 => NotifyUrgency::Low,
            2 => NotifyUrgency::Critical,
            _ => NotifyUrgency::Normal,
        }
    }
}

/// The image a notification shows beside its text (a contact's face, an album cover).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum NoteImage {
    #[default]
    None,
    /// `image-path`.
    Icon(IconRef),
    /// `image-data`, converted to straight RGBA at the boundary. Not persisted (the history file
    /// drops it: `history_file::for_disk`).
    Pixels(Pixels),
}

/// Straight 8-bit RGBA, `width * height * 4` bytes, rows packed. The bytes are shared: the state
/// is copied and compared on every event, and a picture is up to a megabyte.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Pixels {
    pub width: u32,
    pub height: u32,
    #[serde(with = "shared_bytes")]
    pub rgba: Arc<[u8]>,
}

/// Shared bytes in their wire form, a sequence of numbers, as a `Vec<u8>` is.
mod shared_bytes {
    use std::sync::Arc;

    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub(super) fn serialize<S: Serializer>(bytes: &Arc<[u8]>, to: S) -> Result<S::Ok, S::Error> {
        bytes.as_ref().serialize(to)
    }

    pub(super) fn deserialize<'de, D: Deserializer<'de>>(from: D) -> Result<Arc<[u8]>, D::Error> {
        Vec::<u8>::deserialize(from).map(Arc::from)
    }
}

/// The `transient` hint: a transient notification never enters the history.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Transience {
    #[default]
    Kept,
    Transient,
}

/// The `resident` hint: a resident notification is not closed when an action is invoked.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Residency {
    #[default]
    ClosesOnAction,
    Resident,
}

/// The sound hints.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum SoundHint {
    /// None named: the default notification sound.
    #[default]
    Default,
    /// `sound-name`, a freedesktop sound theme name.
    Named(String),
    /// `sound-file`.
    File(PathBuf),
    /// `suppress-sound`: nothing plays, whatever else was named.
    Suppressed,
}

/// `expire_timeout`. design/13 §13.3.6: the banner's hold is its constant
/// whatever the sender asks, except that 0 persists.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum Expiry {
    /// -1 (or any negative): the server's default.
    #[default]
    Default,
    /// 0: never expires.
    Never,
    /// A positive number of milliseconds, recorded and otherwise ignored.
    Ms(u32),
}

impl Expiry {
    /// The wire's `i32`.
    pub fn from_wire(ms: i32) -> Expiry {
        match ms {
            0 => Expiry::Never,
            ms if ms < 0 => Expiry::Default,
            ms => Expiry::Ms(u32::try_from(ms).unwrap_or(u32::MAX)),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{AppRef, Expiry, IconRef, NotifyUrgency, pair_actions};
    use crate::notifications::{ActionKey, AppKey};

    #[test]
    fn wire_words_become_values() {
        assert_eq!(Expiry::from_wire(-1), Expiry::Default);
        assert_eq!(Expiry::from_wire(-7), Expiry::Default);
        assert_eq!(Expiry::from_wire(0), Expiry::Never);
        assert_eq!(Expiry::from_wire(3000), Expiry::Ms(3000));
        const URGENCY: &[(u8, NotifyUrgency)] = &[
            (0, NotifyUrgency::Low),
            (1, NotifyUrgency::Normal),
            (2, NotifyUrgency::Critical),
            (3, NotifyUrgency::Normal),
        ];
        for &(byte, want) in URGENCY {
            assert_eq!(NotifyUrgency::from_wire(byte), want, "{byte}");
        }
        let icons = [
            ("", IconRef::None),
            ("mail-unread", IconRef::Name("mail-unread".to_owned())),
            (
                "file:///tmp/a.png",
                IconRef::File(PathBuf::from("/tmp/a.png")),
            ),
            ("/tmp/b.png", IconRef::File(PathBuf::from("/tmp/b.png"))),
        ];
        for (wire, want) in icons {
            assert_eq!(IconRef::from_wire(wire), want, "{wire:?}");
        }
    }

    #[test]
    fn actions_pair_in_order_and_a_lone_key_is_dropped() {
        let flat: Vec<String> = ["default", "Open", "reply", "Reply", "orphan"]
            .map(str::to_owned)
            .to_vec();
        let actions = pair_actions(&flat);
        assert_eq!(actions.len(), 2);
        assert_eq!(actions[0].key, ActionKey::default_action());
        assert_eq!(actions[1].label, "Reply");
    }

    #[test]
    fn an_app_groups_by_its_desktop_entry_else_its_name() {
        let app = |name: &str, entry: Option<&str>| AppRef {
            name: name.to_owned(),
            icon: IconRef::None,
            desktop_entry: entry.map(str::to_owned),
        };
        let cases = [
            (
                app("Evolution", Some("org.gnome.Evolution")),
                "org.gnome.Evolution",
            ),
            (app("Evolution", None), "Evolution"),
            (app("Evolution", Some("")), "Evolution"),
            (app("", None), ""),
        ];
        for (app, want) in cases {
            assert_eq!(app.key(), AppKey(want.to_owned()), "{app:?}");
        }
    }
}
