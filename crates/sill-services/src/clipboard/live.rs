//! The live loop: its own thread and Wayland connection (as `cosmic_wl`), binding
//! `ext_data_control_manager_v1` else `zwlr_data_control_manager_v1` on the first seat, reading
//! each new selection whole through a pipe (never the primary selection), and offering an
//! entry back as the selection on `Offer`. Lane b fills it (FINDINGS "M9 freeze" F647).
//!
//! The freeze's loop is passive: it answers commands by reducing nothing and never connects, so
//! `Services::spawn` in a test never touches a compositor.

use tokio::sync::{mpsc, watch};

use super::model::{Clipboard, ClipboardCommand, Setup};
use super::wl::manager_interfaces;

/// Run until every control is dropped.
pub(super) async fn run(
    setup: Setup,
    mut commands: mpsc::UnboundedReceiver<ClipboardCommand>,
    tx: watch::Sender<Clipboard>,
) {
    let _ = (&setup, manager_interfaces(), &tx);
    while commands.recv().await.is_some() {}
}
