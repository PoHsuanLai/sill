//! The live loop (lane a, FINDINGS "The M6 fill-in"): connect to the bus `setup` names, follow
//! who holds `org.freedesktop.Notifications`, own it only while the policy says `Own` (never
//! replacing a holder: `AllowReplacement | DoNotQueue`, and ask again when the name falls
//! free), serve [`super::dbus::Server`] while owned, load the history, and run every call and
//! command through [`super::reduce_notifications`], carrying out its effects (signals, sounds,
//! `history_file::save`) and publishing both receivers.
//!
//! Frozen stub: returns at once, so the service is inert (both receivers at their defaults, every
//! command answering `ServiceError::Gone`) until lane a fills it. Nothing here may ever touch the
//! real session bus from a test: tests pass `BusChoice::Address` of a private `dbus-daemon`
//! (`tests/support/private_bus.rs`).

use tokio::sync::{mpsc, watch};

use super::state::{History, Notifications};
use super::{NotificationsCommand, Setup};

/// Run the service until every control is dropped.
pub(super) async fn run(
    setup: Setup,
    live: watch::Sender<Notifications>,
    history: watch::Sender<History>,
    commands: mpsc::UnboundedReceiver<NotificationsCommand>,
) {
    let _ = (setup, live, history, commands);
}
