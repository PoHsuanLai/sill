//! Whether Edit Widgets is open, and on which output: one shared state written by every opener
//! (the notification center's "Edit Widgets", `sill widgets edit`) and followed by every
//! output's Edit Widgets surface onto its own visibility, as `CenterLink` is. It also holds where
//! Edit Widgets' changes go: the daemon's writer saves the keys into `settings.toml` (publishing
//! them at once, as `sill widgets toggle-desktop` does) and the output's places into
//! `desktop-widgets.json`; a test's records them. A link without one changes nothing.

use std::sync::Arc;

use shell_host::OutputId;
use sill_settings::{DesktopWidgets, WidgetKind};
use tokio::sync::watch;

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
#[derive(Debug, Clone)]
pub struct EditWidgetsLink {
    shown: watch::Sender<EditShown>,
    writer: Option<WidgetsWriter>,
    places: Option<PlacementFile>,
}

impl Default for EditWidgetsLink {
    fn default() -> Self {
        EditWidgetsLink::new()
    }
}

impl EditWidgetsLink {
    /// Closed.
    pub fn new() -> EditWidgetsLink {
        EditWidgetsLink {
            shown: watch::Sender::new(EditShown::Closed),
            writer: None,
            places: None,
        }
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

    /// The writer, when the daemon gave one; without it an edit changes nothing.
    pub fn writer(&self) -> Option<&WidgetsWriter> {
        self.writer.as_ref()
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
        *self.shown.borrow()
    }

    /// A receiver for a surface to follow.
    pub fn subscribe(&self) -> watch::Receiver<EditShown> {
        self.shown.subscribe()
    }

    fn set(&self, next: EditShown) {
        self.shown.send_if_modified(|now| {
            let changed = *now != next;
            *now = next;
            changed
        });
    }
}

/// One change: the widget keys whole, and the places of one output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WidgetsWrite {
    /// `widgets.center`.
    pub center: Vec<WidgetKind>,
    /// `widgets.desktop_widgets`.
    pub desktop: Vec<WidgetKind>,
    /// `widgets.desktop`: `On` after an add to the desktop (the widget is to be seen), else as
    /// it was.
    pub shown: DesktopWidgets,
    /// Whose places.
    pub output: OutputKey,
    /// That output's places, every kind it keeps one for.
    pub places: Vec<Placement>,
}

/// The writer, as a root context. Clone is cheap.
#[derive(Clone)]
pub struct WidgetsWriter(pub Arc<dyn Fn(WidgetsWrite) + Send + Sync>);

impl WidgetsWriter {
    /// Write `change`.
    pub fn write(&self, change: WidgetsWrite) {
        (self.0)(change);
    }
}

impl std::fmt::Debug for WidgetsWriter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("WidgetsWriter")
    }
}
