//! The clipboard history's state. Nothing here prints content: the text
//! and image types redact themselves in `Debug`, and only the status types are `Serialize` (history lives in
//! memory only).

use std::fmt;
use std::sync::Arc;

use ds_core::word::Word;
use serde::{Deserialize, Serialize};
use sill_launcher::{ClipKey, Mime, UnixTime};

/// What the launcher and `sill clipboard status` read.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Clipboard {
    pub status: ClipStatus,
    /// Newest first; at most the policy's cap.
    pub entries: Vec<ClipEntry>,
}

/// Whether history is kept, and how the service reaches the clipboard.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ClipStatus {
    pub mode: HistoryMode,
    pub protocol: ClipProtocol,
}

/// `launcher.clipboard_history`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, Word)]
#[serde(rename_all = "snake_case")]
#[word(case = snake)]
pub enum HistoryMode {
    /// Nothing is kept, and the service holds no Wayland connection. The state until the
    /// daemon's first policy.
    #[default]
    Off,
    /// Entries are kept in this process's memory only and are gone when it ends.
    Memory,
}

/// Which data-control protocol the service bound.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClipProtocol {
    /// Not connected (history off, or not started yet).
    #[default]
    NotConnected,
    /// `ext_data_control_manager_v1` (staging).
    Ext,
    /// `zwlr_data_control_manager_v1`, when the compositor has no `ext` one.
    Wlr,
    /// Connected, and the compositor offers neither.
    Unsupported,
}

/// One copied thing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClipEntry {
    pub key: ClipKey,
    pub content: ClipContent,
    pub copied: UnixTime,
}

/// What was copied.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClipContent {
    Text(ClipText),
    Image(ClipImage),
}

/// Copied text. `Debug` prints its length only.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct ClipText(pub Arc<str>);

impl fmt::Debug for ClipText {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ClipText({} bytes)", self.0.len())
    }
}

/// A copied image, as offered (PNG preferred). `Debug` prints its type and size only.
#[derive(Clone, PartialEq, Eq)]
pub struct ClipImage {
    pub mime: Mime,
    pub bytes: Arc<[u8]>,
    pub width: u32,
    pub height: u32,
}

impl fmt::Debug for ClipImage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "ClipImage({}, {}x{}, {} bytes)",
            self.mime.0,
            self.width,
            self.height,
            self.bytes.len()
        )
    }
}

/// Most entries kept (`launcher.clipboard_cap`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct EntryCap(pub u16);

/// The largest image kept, in bytes (`launcher.clipboard_image_max_mb`); a larger one is
/// skipped. Zero keeps no images.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ImageCap(pub u64);

/// What the settings allow.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ClipPolicy {
    pub mode: HistoryMode,
    pub cap: EntryCap,
    pub images: ImageCap,
}

impl Default for ClipPolicy {
    fn default() -> Self {
        ClipPolicy {
            mode: HistoryMode::Off,
            cap: EntryCap(0),
            images: ImageCap(0),
        }
    }
}

/// The reducer's state: what is published and the next key to give.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ClipState {
    pub shown: Clipboard,
    pub policy: ClipPolicy,
    /// The key the next kept entry gets; counts up from 1, never reused.
    pub next: u64,
}
