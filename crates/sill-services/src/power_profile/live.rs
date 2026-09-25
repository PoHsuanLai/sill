//! power-profiles-daemon over the system bus: seeds the three properties, then feeds every
//! change through [`super::reduce_power_profiles`], and carries out [`ProfileCommand`]s.

use tokio::sync::{mpsc, watch};

use super::ProfileCommand;
use super::model::PowerProfiles;

/// Runs until `commands` closes. Without a bus or the daemon, `tx` stays at its default.
///
/// M5 freeze stub, filled by M5 agent (a) svc-session-bt (FINDINGS "M5 freeze"): until then it
/// returns at once, so the service is inert.
pub(super) async fn run(
    tx: watch::Sender<PowerProfiles>,
    commands: mpsc::UnboundedReceiver<ProfileCommand>,
) {
    let _ = (tx, commands);
}
