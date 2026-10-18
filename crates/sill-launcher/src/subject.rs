//! What a row is about, beyond its words: the facts the preview pane and the new row kinds read. A
//! provider fills [`Item::subject`](crate::Item::subject); `Subject::None` is every row that has
//! nothing more to say.
//!
//! Clipboard text travels here as an [`Excerpt`], whose `Debug` never prints it: an `Item` may
//! be debug-printed by a failing test or a log line, and clipboard history is never logged.

use std::fmt;
use std::path::PathBuf;

use almanac_core::MemoryItem;
use docket_core::{EntityRef, UndoId};
use prov::{HelperId, RunId, SessionId, TaskId};
use serde::{Deserialize, Serialize};

use crate::ids::{AppId, ClipKey};
use crate::time::UnixTime;

/// What a row stands for.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum Subject {
    /// Nothing beyond the title (windows, settings, system, calculator).
    #[default]
    None,
    /// An installed app (the apps provider).
    App(AppId),
    /// A file or folder (the files provider).
    File(FileFacts),
    /// A clipboard history entry (the clipboard provider).
    Clip(ClipFacts),
    /// One emoji (the emoji provider).
    Emoji(EmojiFacts),
    /// A web search (the web fallback).
    Web(WebFacts),
    /// A thing an app indexed (the things provider).
    Entity(Box<EntityRef>),
    /// The companion's answer to a prompt, by its task (the companion provider).
    Answer(TaskId),
    /// A helper the person keeps (the helpers provider).
    Helper(HelperFacts),
    /// The row that starts a new helper's card (the helpers provider).
    NewHelper,
    /// A computer-use run (the runs provider).
    Run(RunId),
    /// A row of the undo journal (the companion provider's recent activity).
    Activity(UndoId),
    /// A fact or an event the companion remembers (the memory provider).
    Memory(MemoryItem),
    /// A session the person started in a terminal (the companion provider).
    TerminalSession(SessionId),
}

/// A helper, as its row carries it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HelperFacts {
    /// Which helper.
    pub id: HelperId,
    /// Whether it needs the person.
    pub attention: HelperAttention,
}

/// Whether a helper needs the person.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HelperAttention {
    /// It works, or waits, on its own.
    Calm,
    /// It cannot go on without the person.
    WaitingForYou,
}

/// A file as a file index reports it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileFacts {
    /// Absolute.
    pub path: PathBuf,
    /// `inode/directory` for a folder; guessed from the name when the index gave none.
    pub mime: Mime,
    /// Last modification.
    pub modified: UnixTime,
}

/// A MIME type, lower case: `application/pdf`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Mime(pub String);

impl Mime {
    /// A folder's type (shared-mime-info's).
    pub const FOLDER: &'static str = "inode/directory";
}

/// A clipboard history entry as a row carries it: its key, and enough to draw it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClipFacts {
    pub key: ClipKey,
    pub kind: ClipKind,
    /// When it was copied.
    pub copied: UnixTime,
}

/// What a clipboard entry holds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum ClipKind {
    /// Text: its first lines, and how long the whole is.
    Text { excerpt: Excerpt, chars: CharCount },
    /// An image: its type and size. The bytes stay in the clipboard service.
    Image { mime: Mime, width: u32, height: u32 },
}

/// How many characters a text holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CharCount(pub u32);

/// The first lines of a text, as a preview or a row shows them. `Debug` prints its length only.
#[derive(Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Excerpt(pub String);

impl fmt::Debug for Excerpt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Excerpt({} bytes)", self.0.len())
    }
}

/// One emoji, independent of how its glyph is painted (the system's colour font draws it).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmojiFacts {
    /// The emoji as text, with the skin tone applied when it takes one.
    pub glyph: String,
    /// Its CLDR short name: `grinning face with big eyes`.
    pub name: String,
    pub tone: SkinTone,
}

/// A skin tone modifier (Unicode's five, Fitzpatrick types 1-2 to 6, or none: the yellow
/// default).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SkinTone {
    #[default]
    Default,
    Light,
    MediumLight,
    Medium,
    MediumDark,
    Dark,
}

/// A web search: which engine and the address it opens.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WebFacts {
    /// The engine's host, for the row's detail (`duckduckgo.com`).
    pub engine: String,
    /// The address, the query percent-encoded into the engine's template.
    pub url: String,
}

#[cfg(test)]
mod tests {
    use super::Excerpt;

    #[test]
    fn an_excerpt_never_prints_its_text() {
        let excerpt = Excerpt("hunter2 is my password".to_owned());
        let printed = format!("{excerpt:?}");
        assert!(!printed.contains("hunter2"), "{printed}");
        assert_eq!(printed, "Excerpt(22 bytes)");
    }
}
