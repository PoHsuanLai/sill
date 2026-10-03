//! The preview pane's content, decided from a row alone: Quick Look for the launcher.
//! [`preview_for`] is pure. A file is previewed by the viewer's own parts (`anyview_peek`): the
//! pane sniffs it and peeks at it off the UI thread, so what a file looks like is the viewer's to
//! say and this crate names no format. What a clipboard image is, the pane loads from the
//! clipboard service.

use serde::{Deserialize, Serialize};

use crate::ids::AppId;
use crate::item::Item;
use crate::subject::{ClipFacts, EmojiFacts, Excerpt, FileFacts, Subject, WebFacts};

/// What the pane shows for a row.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum Preview {
    /// Nothing to preview: the pane shows nothing (and Space does not open it).
    #[default]
    None,
    /// Text already in hand.
    Text { excerpt: Excerpt },
    /// A file or a folder: sniffed and peeked at by the viewer's parts, off the UI thread.
    File(FileFacts),
    /// An app: its icon, name and description.
    App {
        app: AppId,
        name: String,
        detail: String,
    },
    /// A clipboard entry: its text, or its image from the clipboard service.
    Clipboard(ClipFacts),
    /// One emoji, large, with its name.
    Emoji(EmojiFacts),
    /// A web search: the engine and the address.
    Web(WebFacts),
}

/// The kind of a preview, for a report or a test: its variant's name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PreviewKind {
    None,
    Text,
    File,
    App,
    Clipboard,
    Emoji,
    Web,
}

impl Preview {
    /// This preview's kind.
    pub fn kind(&self) -> PreviewKind {
        match self {
            Preview::None => PreviewKind::None,
            Preview::Text { .. } => PreviewKind::Text,
            Preview::File(_) => PreviewKind::File,
            Preview::App { .. } => PreviewKind::App,
            Preview::Clipboard(_) => PreviewKind::Clipboard,
            Preview::Emoji(_) => PreviewKind::Emoji,
            Preview::Web(_) => PreviewKind::Web,
        }
    }
}

/// How much of a text an excerpt keeps.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExcerptLimit {
    /// At most this many lines.
    pub lines: u32,
    /// At most this many characters in all.
    pub chars: u32,
}

impl ExcerptLimit {
    /// What the pane shows: 40 lines, 4000 characters.
    pub const PANE: ExcerptLimit = ExcerptLimit {
        lines: 40,
        chars: 4000,
    };
}

/// What the pane shows for `item`, from its subject and title alone.
///
/// Rules (`preview_tests` pins them): an `App` subject is `App` with the row's title and subtitle;
/// a `File` is `File`, whatever its type; a `Clip` is `Clipboard`; an `Emoji` is `Emoji`; a `Web`
/// is `Web`; `Subject::None` is `None`.
pub fn preview_for(item: &Item) -> Preview {
    match &item.subject {
        Subject::None => Preview::None,
        Subject::App(app) => Preview::App {
            app: app.clone(),
            name: item.title.clone(),
            detail: item.subtitle.clone(),
        },
        Subject::File(facts) => Preview::File(facts.clone()),
        Subject::Clip(facts) => Preview::Clipboard(facts.clone()),
        Subject::Emoji(facts) => Preview::Emoji(facts.clone()),
        Subject::Web(facts) => Preview::Web(facts.clone()),
    }
}

/// The first lines of `bytes` as the pane shows them: decoded as UTF-8 (invalid sequences
/// replaced), cut at `limit` on a character boundary, trailing whitespace trimmed; binary data
/// (a NUL in the first 1024 bytes) is an empty excerpt.
pub fn excerpt_of(bytes: &[u8], limit: ExcerptLimit) -> Excerpt {
    let head = &bytes[..bytes.len().min(BINARY_PROBE)];
    if head.contains(&0) {
        return Excerpt::default();
    }
    let text = String::from_utf8_lossy(bytes);
    let lines = usize::try_from(limit.lines).unwrap_or(usize::MAX);
    let chars = usize::try_from(limit.chars).unwrap_or(usize::MAX);
    let cut: String = text
        .split_inclusive('\n')
        .take(lines)
        .collect::<String>()
        .chars()
        .take(chars)
        .collect();
    Excerpt(cut.trim_end().to_owned())
}

/// How far into a file a NUL marks it binary.
const BINARY_PROBE: usize = 1024;

#[cfg(test)]
#[path = "preview_tests.rs"]
mod preview_tests;
