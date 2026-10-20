//! Carries out Next Input Source: finds the framework on the bus at the time, then switches it.
//! Tests feed it a private bus with a stand-in controller, never a real fcitx5.

use sill_model::input_source::{InputCommand, InputRefusal, InputSwitch};
use tokio::sync::{mpsc, watch};
use zbus::Connection;

use super::{fcitx, ibus};
use crate::env::{Bus, IbusRoute};
use crate::error::ServiceError;
use crate::owner;

/// Runs until `commands` closes. Without a session bus the state stays idle.
pub(crate) async fn run(
    bus: Bus,
    ibus: IbusRoute,
    tx: watch::Sender<InputSwitch>,
    mut commands: mpsc::UnboundedReceiver<InputCommand>,
) -> Result<(), ServiceError> {
    let connection = bus.connect_waiting().await?;
    while let Some(InputCommand::Next) = commands.recv().await {
        let now = match switch(&connection, &ibus).await {
            Ok(name) => InputSwitch::Switched(name),
            Err(refusal) => {
                eprintln!("sill-services: input_source: {refusal:?}");
                InputSwitch::Refused(refusal)
            }
        };
        tx.send_replace(now);
    }
    Ok(())
}

/// The framework running now, and what switching it came to: fcitx5 on the session bus, else
/// IBus on its own bus.
async fn switch(connection: &Connection, ibus_route: &IbusRoute) -> Result<String, InputRefusal> {
    if owner::current(connection, fcitx::NAME).await.is_some() {
        return fcitx::switch(connection).await;
    }
    match ibus::connect(ibus_route).await {
        Some(own) => ibus::switch(&own).await,
        None => Err(InputRefusal::NoFramework),
    }
}
