//! logind over the system bus: probes `CanSuspend`, `CanReboot` and `CanPowerOff` (and this
//! session's object for Lock and Log Out), publishes them through [`super::reduce_session`], and
//! carries out [`SessionCommand`]s.

use tokio::sync::{mpsc, watch};

use super::SessionCommand;
use super::model::Session;

/// Runs until `commands` closes. Without a bus or logind, `tx` stays at its default.
///
/// M5 freeze stub, filled by M5 agent (a) svc-session-bt (FINDINGS "M5 freeze"): until then it
/// returns at once, so the service is inert.
pub(super) async fn run(
    tx: watch::Sender<Session>,
    commands: mpsc::UnboundedReceiver<SessionCommand>,
) {
    let _ = (tx, commands);
}
