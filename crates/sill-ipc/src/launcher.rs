//! The launcher's requests: the clipboard history's status and clear, and the debug requests an
//! acceptance script drives the launcher with (a key, and the rows as shown). Reports carry counts
//! and kinds only: never a clipboard entry's content.

use serde::{Deserialize, Serialize};

/// `sill clipboard ...`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum ClipboardRequest {
    /// Whether history is kept, how, and how many entries: answered with `Response::Clipboard`.
    Status,
    /// Forget every entry now.
    Clear,
}

/// The clipboard history as `sill clipboard status` prints it (`sill_services::clipboard`'s
/// mirror; this crate may not depend on sill-services).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ClipboardReport {
    pub mode: ReportClipMode,
    pub protocol: ReportClipProtocol,
    pub entries: u32,
}

/// `launcher.clipboard_history` as the service was last told it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReportClipMode {
    Off,
    Memory,
}

/// The data-control protocol the service bound.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReportClipProtocol {
    NotConnected,
    Ext,
    Wlr,
    Unsupported,
}

/// A key for the launcher, as if typed (`sill debug launcher-key`; the demo's script keys and
/// the preview pane's). Acted on only by a daemon built with `debug`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FakeLauncherKey {
    Up,
    Down,
    Enter,
    Escape,
    Tab,
    ShiftTab,
    /// Ctrl+K.
    Actions,
    /// Space: the preview pane (Quick Look).
    Space,
    /// Ctrl+Y (⌘Y): the preview pane from anywhere.
    QuickLook,
    Right,
    Left,
    /// Ctrl+R (⌘R): the selected row's reveal.
    Reveal,
}

/// The launcher as shown on the output it is open on (`sill debug launcher-rows`): each section
/// with its row titles, the selection, and the preview pane. Answered only by a daemon built
/// with `debug`; clipboard rows' titles are replaced by `"<clipboard>"`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LauncherRows {
    pub query: String,
    pub sections: Vec<RowsSection>,
    /// The selected row, counted over every section's rows.
    pub selected: u32,
    pub preview: ReportPreview,
}

/// One section as shown.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RowsSection {
    pub title: String,
    pub rows: Vec<String>,
    /// Rows hidden behind "Show More".
    pub hidden: u32,
}

/// The preview pane.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum ReportPreview {
    Hidden,
    /// Shown, with the kind of preview (`sill_launcher::PreviewKind`'s spelling).
    Shown(ReportPreviewKind),
}

/// `sill_launcher::PreviewKind`'s mirror.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReportPreviewKind {
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
