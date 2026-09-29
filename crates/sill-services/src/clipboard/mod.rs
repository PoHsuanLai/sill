//! Clipboard history for the launcher (quire design/09 H4, integrated capture: what was copied is
//! one keystroke away in the launcher).
//!
//! The service keeps the last N copied texts and images **in this process's memory only**:
//! nothing is written to disk, nothing is logged (the content types redact themselves in
//! `Debug`), offers a password manager marks are never read (`SECRET_HINTS`), and with
//! `launcher.clipboard_history = "off"` it holds no connection and no entries. It is passive
//! until the daemon's first [`ClipboardCommand::Policy`], so a test never reads the session's
//! clipboard. Live tests connect to a nested compositor by socket
//! ([`WaylandRoute::Socket`](crate::env::WaylandRoute::Socket))
//! with their own data only.

mod backend;

use sill_model::clipboard::{Clipboard, ClipboardCommand};

use crate::env::Env;
use crate::error::ServiceError;
use crate::service::{Service, ServiceIo};

/// The clipboard history as a [`Service`], on `env`'s compositor. Passive until the daemon's
/// first policy.
#[derive(Debug)]
pub enum ClipboardService {}

impl Service for ClipboardService {
    const NAME: &'static str = "clipboard";
    type State = Clipboard;
    type Command = ClipboardCommand;
    type Deps = ();

    fn run(
        env: &Env,
        (): (),
        io: ServiceIo<Self>,
    ) -> impl Future<Output = Result<(), ServiceError>> + Send + 'static {
        let display = env.wayland.clone();
        async move {
            backend::run(display, io.commands, io.state).await;
            Ok(())
        }
    }
}
