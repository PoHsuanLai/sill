//! What `sill debug launcher-rows` reaches through the [`LauncherLink`](super::LauncherLink): the
//! rows the panel shows. The panel publishes [`ShownRows`] after every
//! render while it is open and `None` when it hides; clipboard rows' titles never reach it
//! (they are `<clipboard>` here already, so no clipboard text leaves the panel).

use sill_launcher::PreviewKind;

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
