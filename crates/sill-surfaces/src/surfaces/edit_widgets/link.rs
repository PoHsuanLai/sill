//! Whether Edit Widgets is open, and on which output: one shared state written by every opener
//! (the notification center's "Edit Widgets", `sill widgets edit`) and followed by every
//! output's Edit Widgets surface onto its own visibility, as `CenterLink` is.

use shell_host::OutputId;
use tokio::sync::watch;

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
        }
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
