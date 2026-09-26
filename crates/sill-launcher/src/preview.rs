//! The preview pane's content, decided from a row alone (M9, FINDINGS "M9 freeze" F644): Quick
//! Look for the launcher. [`preview_for`] is pure; what it names on disk (a text file's first
//! lines, a PDF's first page, a clipboard image's bytes) the pane loads off the UI thread, and a
//! PDF page is drawn by quire's `PdfThumb` from the path (sill never rasterises).

use std::path::PathBuf;

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
    match &item.subject {
        Subject::None => Preview::None,
        Subject::App(app) => Preview::App {
            app: app.clone(),
            name: item.title.clone(),
            detail: item.subtitle.clone(),
        },
        Subject::File(facts) => file_preview(facts),
        Subject::Clip(facts) => Preview::Clipboard(facts.clone()),
        Subject::Emoji(facts) => Preview::Emoji(facts.clone()),
        Subject::Web(facts) => Preview::Web(facts.clone()),
    }
}

/// Image types the renderer decodes.
const DECODED_IMAGES: [&str; 6] = [
    "image/png",
    "image/jpeg",
    "image/svg+xml",
    "image/webp",
    "image/gif",
    "image/bmp",
];

/// `application/*` types that are text in all but name: source, config and data files.
const TEXT_APPLICATIONS: [&str; 14] = [
    "application/json",
    "application/xml",
    "application/toml",
    "application/x-toml",
    "application/yaml",
    "application/x-yaml",
    "application/javascript",
    "application/x-shellscript",
    "application/x-sh",
    "application/sql",
    "application/x-desktop",
    "application/x-perl",
    "application/x-python",
    "application/x-ruby",
];

fn file_preview(facts: &FileFacts) -> Preview {
    let mime = facts.mime.0.as_str();
    let path = facts.path.clone();
    match mime {
        "application/pdf" => Preview::Pdf {
            path,
            page: PageIndex::FIRST,
        },
        m if DECODED_IMAGES.contains(&m) => Preview::Image(path),
        m if m.starts_with("text/") || TEXT_APPLICATIONS.contains(&m) => Preview::TextFile(path),
        _ => Preview::File(facts.clone()),
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
