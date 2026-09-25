//! The focus mode, published and settable: a watch of [`FocusMode`] that follows the settled
//! settings, and a command channel whose `Set` rewrites `settings.toml`. The daemon starts one
//! ([`spawn_focus`]) and hands it to every surface as a root context beside `SettingsFeed`
//! (`sill-surfaces::hooks::use_focus_mode`); a later notifications server reads the same
//! receiver.
//!
//! Setting goes through the file, never around it: the write is seen by the settings watch like
//! any other edit, the feed settles, and the mode follows. So the file stays the one truth, and a
//! hand edit and a toggle cannot disagree.

use std::path::PathBuf;

use tokio::runtime::Handle;
use tokio::sync::{mpsc, watch};

use crate::file::ShellFile;
use crate::focus::{FocusMode, focus_mode};

/// Whether a set reached the focus task.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Delivery {
    Queued,
    /// The task has stopped (the daemon is exiting); the mode is unchanged.
    Gone,
}

/// The focus mode's receiver and its setter. Clone is cheap. A watch receiver and a sender,
/// never a Dioxus signal.
#[derive(Debug, Clone)]
pub struct FocusLink {
    mode: watch::Receiver<FocusMode>,
    set: mpsc::UnboundedSender<FocusMode>,
}

impl FocusLink {
    /// A link from its halves (the task's, or a test's).
    pub fn new(mode: watch::Receiver<FocusMode>, set: mpsc::UnboundedSender<FocusMode>) -> Self {
        FocusLink { mode, set }
    }

    /// A link fixed at `mode` whose sets go nowhere, for a headless render or a test.
    pub fn fixed(mode: FocusMode) -> FocusLink {
        let (_tx, rx) = watch::channel(mode);
        let (set, _commands) = mpsc::unbounded_channel();
        FocusLink { mode: rx, set }
    }

    /// A receiver of the mode in effect.
    pub fn mode(&self) -> watch::Receiver<FocusMode> {
        self.mode.clone()
    }

    /// Ask for `mode`: stored in `settings.toml`, published once the file settles.
    pub fn set(&self, mode: FocusMode) -> Delivery {
        match self.set.send(mode) {
            Ok(()) => Delivery::Queued,
            Err(_) => Delivery::Gone,
        }
    }
}

/// Start the focus task on `rt`: its mode follows `settings` through [`focus_mode`], and each
/// set loads the file in `dir`, stores the mode with `crate::focus::with_focus_mode`, and saves
/// it (`crate::save`). Without a `dir` sets are dropped (logged) and the mode still follows the
/// feed.
///
/// M5 freeze stub, filled by M5 agent (b) svc-media-wifi-focus (FINDINGS "M5 freeze"): until then
/// the mode is fixed at the feed's current value and sets go nowhere.
pub fn spawn_focus(
    rt: &Handle,
    settings: watch::Receiver<ShellFile>,
    dir: Option<PathBuf>,
) -> FocusLink {
    let _ = (rt, dir);
    let now = focus_mode(&settings.borrow());
    FocusLink::fixed(now)
}
