//! Carries out Next Input Source: finds the framework on the bus at the time, then switches it.
//! Tests feed it a private bus with a stand-in controller, never a real fcitx5.

use sill_model::input_source::{InputCommand, InputFramework, InputRefusal, InputSwitch};
use tokio::sync::{mpsc, watch};
use zbus::Connection;

use super::fcitx;
use crate::env::Bus;
use crate::error::ServiceError;
use crate::owner;

/// IBus's name on the session bus (the portal it offers sandboxed apps); its own bus is found
/// through an address file, which the shell does not read.
const IBUS_PORTAL: &str = "org.freedesktop.portal.IBus";

/// Runs until `commands` closes. Without a session bus the state stays idle.
pub(crate) async fn run(
    bus: Bus,
    tx: watch::Sender<InputSwitch>,
    mut commands: mpsc::UnboundedReceiver<InputCommand>,
) -> Result<(), ServiceError> {
    let connection = bus.connect_waiting().await?;
    while let Some(InputCommand::Next) = commands.recv().await {
        let now = match switch(&connection).await {
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

/// The framework on the bus now, and what switching it came to.
async fn switch(connection: &Connection) -> Result<String, InputRefusal> {
    if owner::current(connection, fcitx::NAME).await.is_some() {
        return fcitx::switch(connection).await;
    }
    if owner::current(connection, IBUS_PORTAL).await.is_some() {
        return Err(InputRefusal::Unsupported(InputFramework::IBus));
    }
    Err(InputRefusal::NoFramework)
}
