//! Where Edit Widgets' changes go: the daemon's writer saves the keys into `settings.toml`
//! (publishing them at once, as `sill widgets toggle-desktop` does) and the output's places into
//! `desktop-widgets.json`; a test's records them. A root without one changes nothing and says
//! so.

use std::sync::Arc;

use sill_settings::{DesktopWidgets, WidgetKind};

use crate::surfaces::desktop_widgets::{OutputKey, Placement};

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
