//! The preview pane's content, decided from a row alone (M9, FINDINGS "M9 freeze" F644): Quick
//! Look for the launcher. [`preview_for`] is pure; what it names on disk (a text file's first
//! lines, a PDF's first page, a clipboard image's bytes) the pane loads off the UI thread, and a
//! PDF page is drawn by quire's `PdfThumb` from the path (sill never rasterises).

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::ids::AppId;
use crate::item::Item;
use crate::subject::{ClipFacts, EmojiFacts, Excerpt, FileFacts, WebFacts};

/// What the pane shows for a row.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum Preview {
    /// Nothing to preview: the pane shows nothing (and Space does not open it).
    #[default]
    None,
    /// An image file (`image/*` the renderer decodes: PNG, JPEG, SVG, WebP, GIF).
    Image(PathBuf),
    /// Text already in hand.
    Text { excerpt: Excerpt },
    /// A text file: its first lines, read by the pane ([`excerpt_of`] over its head).
    TextFile(PathBuf),
    /// A PDF's page, drawn by quire's `PdfThumb`.
    Pdf { path: PathBuf, page: PageIndex },
    /// Any other file or a folder: its facts (name, kind, size, modified, where).
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

/// A page, from 0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PageIndex(pub u32);

impl PageIndex {
    pub const FIRST: PageIndex = PageIndex(0);
}

/// The kind of a preview, for a report or a test: its variant's name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PreviewKind {
    None,
    Image,
    Text,
    TextFile,
    Pdf,
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
            Preview::Image(_) => PreviewKind::Image,
            Preview::Text { .. } => PreviewKind::Text,
            Preview::TextFile(_) => PreviewKind::TextFile,
            Preview::Pdf { .. } => PreviewKind::Pdf,
            Preview::File(_) => PreviewKind::File,
            Preview::App { .. } => PreviewKind::App,
            Preview::Clipboard(_) => PreviewKind::Clipboard,
            Preview::Emoji(_) => PreviewKind::Emoji,
            Preview::Web(_) => PreviewKind::Web,
        }
    }
}

/// How much of a text the pane shows.
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
/// Rules (lane d fills; `preview_tests` pins them): an `App` subject is `App` with the row's
/// title and subtitle; a `File` is `Pdf` at the first page for `application/pdf`, `Image` for an
/// `image/*` the renderer decodes, `TextFile` for `text/*` and the common source and config
/// types, `File` for everything else and for folders; a `Clip` is `Clipboard`; an `Emoji` is
/// `Emoji`; a `Web` is `Web`; `Subject::None` is `None`.
pub fn preview_for(item: &Item) -> Preview {
    let _ = item;
    todo!("M9 fill: d-preview")
}

/// The first lines of `bytes` as the pane shows them: decoded as UTF-8 (invalid sequences
/// replaced), cut at `limit` on a character boundary, trailing whitespace trimmed; binary data
/// (a NUL in the first 1024 bytes) is an empty excerpt.
pub fn excerpt_of(bytes: &[u8], limit: ExcerptLimit) -> Excerpt {
    let _ = (bytes, limit);
    todo!("M9 fill: d-preview")
}

#[cfg(test)]
#[path = "preview_tests.rs"]
mod preview_tests;
