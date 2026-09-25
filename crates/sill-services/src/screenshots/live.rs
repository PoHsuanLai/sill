//! The screenshots task (M10 lane c fills it): on `Configure`, resolve the folders
//! (`folders_of`, the Pictures folder from `dirs::picture_dir`), keep those that exist, watch
//! them with `notify` (a file closed after writing, or renamed into place, whose name
//! `is_screenshot_name`), and publish `Watching`; run each event and command through
//! `reduce_shots` and carry out its effects: `MakePreview` on the blocking pool (`image`, a PNG
//! at most 480 px on its long side, into `$XDG_CACHE_HOME/sill/shots/preview_name`), `Open`
//! (`xdg-open`), `Trash` (`gio trash`), `RunTool` (`tool_command` into `save_folder`, a child
//! whose exit is `TakeEnded`). Ends when every receiver is gone.
//!
//! Frozen as a stub that returns at once, so `Shots` stays at its default (nothing watched).

use tokio::sync::{mpsc, watch};

use super::ShotCommand;
use super::reduce::Shots;

pub(super) async fn run(tx: watch::Sender<Shots>, commands: mpsc::UnboundedReceiver<ShotCommand>) {
    let _ = (tx, commands);
}
