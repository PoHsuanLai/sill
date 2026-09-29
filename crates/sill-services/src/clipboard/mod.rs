//! Clipboard history for the launcher (quire design/09 H4, integrated capture: what was copied is
//! one keystroke away in the launcher).
//!
//! The service keeps the last N copied texts and images **in this process's memory only**:
//! nothing is written to disk, nothing is logged (the content types redact themselves in
//! `Debug`), offers a password manager marks are never read ([`SECRET_HINTS`]), and with
//! `launcher.clipboard_history = "off"` it holds no connection and no entries. It is passive
//! until the daemon's first [`ClipboardCommand::Policy`], so a test never reads the session's
//! clipboard. Live tests connect to a nested compositor by socket ([`ClipDisplay::Socket`])
//! with their own data only.

mod content;
mod live;
mod model;
mod offer;
mod pipes;
mod reduce;
mod wire;
mod wire_dispatch;
mod wire_state;
mod wl;

pub use model::{
    ClipContent, ClipDisplay, ClipEffect, ClipEntry, ClipEvent, ClipImage, ClipPolicy,
    ClipProtocol, ClipState, ClipStatus, ClipText, Clipboard, ClipboardCommand, EntryCap,
    HistoryMode, ImageCap, Setup, Skipped,
};
pub use offer::{ImagesKept, MimePick, OfferVerdict, SECRET_HINTS, offer_verdict};
pub use reduce::reduce_clipboard;

use tokio::runtime::Handle;
use tokio::sync::watch;

use crate::control::Control;

/// Start the service on the session's compositor.
pub fn spawn(rt: &Handle) -> (watch::Receiver<Clipboard>, Control<ClipboardCommand>) {
    spawn_with(rt, Setup::default())
}

/// [`spawn`] with a chosen compositor.
pub fn spawn_with(
    rt: &Handle,
    setup: Setup,
) -> (watch::Receiver<Clipboard>, Control<ClipboardCommand>) {
    let (tx, rx) = watch::channel(Clipboard::default());
    let (control, commands) = Control::channel();
    rt.spawn(live::run(setup, commands, tx));
    (rx, control)
}
