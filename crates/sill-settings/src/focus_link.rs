//! The focus mode, published and settable: a watch of [`FocusMode`] that follows the settled
//! settings, and a command channel whose `Set` rewrites `settings.toml`. The daemon starts one
//! ([`spawn_focus`]) and hands it to every surface as a root context beside `SettingsFeed`
//! (`sill-surfaces::hooks::use_focus_mode`); a later notifications server reads the same
//! receiver.
//!
//! Setting goes through the file, never around it: the write is seen by the settings watch like
//! any other edit, the feed settles, and the mode follows. So the file stays the one truth, and a
//! hand edit and a toggle cannot disagree.

use std::path::{Path, PathBuf};

use tokio::runtime::Handle;
use tokio::sync::{mpsc, watch};

use crate::file::{ShellFile, load, save};
use crate::focus::{FocusMode, focus_mode, with_focus_mode};

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
pub fn spawn_focus(
    rt: &Handle,
    settings: watch::Receiver<ShellFile>,
    dir: Option<PathBuf>,
) -> FocusLink {
    let now = focus_mode(&settings.borrow());
    let (mode_tx, mode_rx) = watch::channel(now);
    let (set_tx, set_rx) = mpsc::unbounded_channel();
    rt.spawn(run(settings, dir, mode_tx, set_rx));
    FocusLink::new(mode_rx, set_tx)
}

/// Follows `settings` into `mode_tx` and carries out every queued set, until either channel
/// closes.
async fn run(
    mut settings: watch::Receiver<ShellFile>,
    dir: Option<PathBuf>,
    mode_tx: watch::Sender<FocusMode>,
    mut sets: mpsc::UnboundedReceiver<FocusMode>,
) {
    loop {
        tokio::select! {
            changed = settings.changed() => {
                if changed.is_err() {
                    return;
                }
                let mode = focus_mode(&settings.borrow());
                if mode_tx.send(mode).is_err() {
                    return;
                }
            }
            set = sets.recv() => match set {
                Some(mode) => store(dir.as_deref(), mode),
                None => return,
            }
        }
    }
}

/// Writes `mode` into `dir`'s `settings.toml`, every other key as it was; without a `dir` (a
/// headless render or a test with no file to write) the set is dropped.
fn store(dir: Option<&Path>, mode: FocusMode) {
    let Some(dir) = dir else {
        eprintln!("sill-settings: focus: no settings directory, set({mode:?}) dropped");
        return;
    };
    let file = with_focus_mode(load(dir), mode);
    if let Err(error) = save(dir, &file) {
        eprintln!("sill-settings: focus: could not save settings.toml: {error}");
    }
}
