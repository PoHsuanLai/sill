//! Whether Edit Widgets is open, and on which output: one shared state written by every opener
//! (the notification center's "Edit Widgets", the desktop's menu, `sill widgets edit`) and
//! followed by every output's Edit Widgets surface onto its own visibility, as `CenterLink` is.
//! It also holds where the desktop's changes go: the daemon's writer saves the notification
//! center's keys and the desktop's view into `settings.toml` (publishing them at once, as `sill
//! widgets toggle-desktop` does) and an output's widgets and places into `desktop-widgets.json`;
//! a test's records them. A write is announced on [`EditWidgetsLink::written`], so each output's
//! desktop layer and sheet read the file again. A link without a writer changes nothing.

use std::sync::Arc;

use shell_host::OutputId;
use sill_settings::{DesktopWidgets, Px, WidgetKind};
use tokio::sync::watch;

use super::Shared;

use super::PlacementFile;
use crate::widgets::grid::{OutputKey, Placement};

/// Edit Widgets' state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum EditShown {
    #[default]
    Closed,
    Open(OutputId),
}

/// The openers' half and every surface's receiver. Clone is cheap.
#[derive(Debug, Clone, Default)]
pub struct EditWidgetsLink {
    shown: Shared<EditShown>,
    written: Shared<Written>,
    writer: Option<WidgetsWriter>,
    places: Option<PlacementFile>,
}

impl EditWidgetsLink {
    /// Closed.
    pub fn new() -> EditWidgetsLink {
        EditWidgetsLink::default()
    }

    /// The same link with `writer` saving Edit Widgets' changes.
    pub fn with_writer(mut self, writer: WidgetsWriter) -> EditWidgetsLink {
        self.writer = Some(writer);
        self
    }

    /// The same link with the desktop's places read from `places`.
    pub fn with_places(mut self, places: PlacementFile) -> EditWidgetsLink {
        self.places = Some(places);
        self
    }

    /// Carry out `change` through the writer and announce it on [`EditWidgetsLink::written`];
    /// without a writer nothing is saved and nothing is announced.
    pub fn write(&self, change: WidgetsChange) {
        match &self.writer {
            Some(writer) => {
                writer.write(change);
                self.written.modify(|serial| *serial = serial.next());
            }
            None => eprintln!("sill: the desktop has no writer; nothing was saved"),
        }
    }

    /// A receiver that changes after each [`EditWidgetsLink::write`], for a surface that reads
    /// what the writer saved.
    pub fn written(&self) -> watch::Receiver<Written> {
        self.written.subscribe()
    }

    /// The file the desktop's places are in, when the daemon gave one.
    pub fn places(&self) -> Option<&PlacementFile> {
        self.places.as_ref()
    }

    /// Open on `output` (moving it there if it is open elsewhere).
    pub fn open(&self, output: OutputId) {
        self.set(EditShown::Open(output));
    }

    /// Close it, wherever it is.
    pub fn close(&self) {
        self.set(EditShown::Closed);
    }

    /// The state now.
    pub fn now(&self) -> EditShown {
        self.shown.now()
    }

    /// A receiver for a surface to follow.
    pub fn subscribe(&self) -> watch::Receiver<EditShown> {
        self.shown.subscribe()
    }

    fn set(&self, next: EditShown) {
        self.shown.set(next);
    }
}

/// How many writes the link has carried out: a receiver of it sees each one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Written(pub u32);

impl Written {
    /// The count after one more write.
    pub fn next(self) -> Written {
        Written(self.0.wrapping_add(1))
    }
}

/// What the desktop asks to have saved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WidgetsChange {
    /// Edit Widgets' edit of one output.
    Edit(WidgetsWrite),
    /// The desktop menu's choice of the grid.
    View(DesktopView),
}

/// The widget grid's measures, as the desktop menu's View Options set them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DesktopView {
    /// `widgets.desktop_cell_px`.
    pub cell: Px,
    /// `widgets.desktop_gap_px`.
    pub gap: Px,
}

/// One edit: the notification center's widgets, and what one output's desktop shows and where.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WidgetsWrite {
    /// `widgets.center`.
    pub center: Vec<WidgetKind>,
    /// The widgets `output` shows: the output's own list in the state file.
    pub desktop: Vec<WidgetKind>,
    /// `widgets.desktop`: `On` after an add to the desktop (the widget is to be seen), else as
    /// it was.
    pub shown: DesktopWidgets,
    /// Whose widgets and places.
    pub output: OutputKey,
    /// That output's places, every kind it keeps one for.
    pub places: Vec<Placement>,
}

/// The writer, as a root context. Clone is cheap.
#[derive(Clone)]
pub struct WidgetsWriter(pub Arc<dyn Fn(WidgetsChange) + Send + Sync>);

impl WidgetsWriter {
    /// Write `change`.
    pub fn write(&self, change: WidgetsChange) {
        (self.0)(change);
    }
}

impl std::fmt::Debug for WidgetsWriter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("WidgetsWriter")
    }
}
