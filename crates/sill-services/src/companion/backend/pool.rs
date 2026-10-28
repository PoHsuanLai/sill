//! Keeping the models Automatic may use for one helper (`ai.auto.pool.<slot>.helper.<id>`).
//!
//! The pool goes to inferd through porter's client, which is the one writer of that key. An empty
//! pool takes the helper off Automatic. A refusal or an unreachable service is logged, and the
//! person is told it could not be set, rather than seeing Automatic switched on that is not.

use std::collections::BTreeSet;

use porter_client::{Accounts, DbusTransport};
use porter_infer::{ModelRef, Slot};
use prov::HelperId;
use sill_model::companion::HelpersMsg;
use zbus::Connection;

/// The pool of `helper`, kept; says whether it was.
pub(super) async fn keep(
    connection: &Connection,
    helper: &HelperId,
    pool: &BTreeSet<ModelRef>,
) -> HelpersMsg {
    let models: Vec<ModelRef> = pool.iter().cloned().collect();
    let accounts = Accounts::over(DbusTransport::over(connection.clone()));
    let written = match accounts.set_helper_pool(Slot::Text, helper, &models).await {
        Ok(()) => true,
        Err(why) => {
            eprintln!("sill: companion: helpers: the models Automatic may use for {helper}: {why}");
            false
        }
    };
    HelpersMsg::PoolWritten(written)
}
