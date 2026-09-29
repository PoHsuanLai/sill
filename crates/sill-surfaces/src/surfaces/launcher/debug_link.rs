//! What `sill debug launcher-key` and `sill debug launcher-rows` reach through the
//! [`LauncherLink`](super::LauncherLink): a key the panel steps as if
//! it were pressed, and the rows the panel shows. The panel publishes [`ShownRows`] after every
//! render while it is open and `None` when it hides; clipboard rows' titles never reach it
//! (they are `<clipboard>` here already, so no clipboard text leaves the panel).

use sill_launcher::PreviewKind;

/// A key for the open launcher, as the panel steps it (`sill_ipc::FakeLauncherKey`'s mirror:
/// this crate does not depend on sill-ipc).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LinkKey {
    Up,
    Down,
    Enter,
    Escape,
    Tab,
    ShiftTab,
    /// Ctrl+K.
    Actions,
    /// Space: Quick Look while browsing, else a space in the query.
    Space,
    /// ⌘Y.
    QuickLook,
    /// → with the caret where the panel last knew it.
    Right,
    Left,
    /// ⌘R on the selected row.
    Reveal,
}

/// A pressed [`LinkKey`] and its count, so the same key twice is two presses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct LinkPress {
    pub count: u64,
    pub key: Option<LinkKey>,
}

/// The launcher as the panel shows it.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ShownRows {
    pub query: String,
    pub sections: Vec<ShownRowsSection>,
    /// The selected line, over every section's lines (a "Show More" stand-in line included).
    pub selected: u32,
    /// The pane: `None` while hidden, else the kind of what it shows.
    pub preview: Option<PreviewKind>,
}

/// One section as shown: its heading, its lines' titles (a clipboard row's is `<clipboard>`;
/// the "Show More"/"Show Less" stand-in is a line until quire's header action), and how
/// many rows it hides.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ShownRowsSection {
    pub title: String,
    pub rows: Vec<String>,
    pub hidden: u32,
}

/// What a clipboard row's title reads as outside the panel.
pub const CLIPBOARD_TITLE: &str = "<clipboard>";
