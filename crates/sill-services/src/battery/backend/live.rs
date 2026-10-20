//! UPower's display device over the system bus: seeds a real first value, then feeds every
//! property change through [`step`].

use sill_model::battery::{Battery, BatteryCommand, BatteryEvent, Charge, step};
use sill_model::units::{Percent, Presence, Seconds};
use tokio::sync::{mpsc, watch};
use upower_dbus::{BatteryState, DeviceProxy, UPowerProxy};
use zbus::Connection;

use crate::env::Bus;
use crate::error::ServiceError;
use crate::owner::{self, Ended};
use crate::stream::next;

const SERVICE: &str = "org.freedesktop.UPower";

/// Follows UPower's display device and runs until `commands` closes (the `Control` was
/// dropped). Without UPower, `tx` is left at its seeded default (absent); UPower starting late
/// or restarting is followed through `NameOwnerChanged` ([`crate::owner`]).
pub(crate) async fn run(
    bus: Bus,
    tx: watch::Sender<Battery>,
    mut commands: mpsc::UnboundedReceiver<BatteryCommand>,
) -> Result<(), ServiceError> {
    let connection = bus.connect_waiting().await?;
    owner::follow_owner(
        &connection,
        SERVICE,
        &mut commands,
        async |commands| session(&connection, &tx, commands).await,
        |_| apply(&tx, BatteryEvent::Presence(Presence::Absent)),
    )
    .await;
    Ok(())
}

/// One UPower lifetime: seed, then follow every property change. [`Ended::Commands`] when `commands`
/// closed, [`Ended::Unreadable`] when UPower did not answer.
async fn session(
    connection: &Connection,
    tx: &watch::Sender<Battery>,
    commands: &mut mpsc::UnboundedReceiver<BatteryCommand>,
) -> Ended {
    let Ok(upower) = UPowerProxy::new(connection).await else {
        return Ended::Unreadable;
    };
    let Ok(device) = upower.get_display_device().await else {
        return Ended::Unreadable;
    };

    seed(tx, &device).await;

    let mut presence = device.receive_is_present_changed().await;
    let mut level = device.receive_percentage_changed().await;
    let mut charge = device.receive_state_changed().await;
    let mut to_empty = device.receive_time_to_empty_changed().await;
    let mut to_full = device.receive_time_to_full_changed().await;

    loop {
        tokio::select! {
            Some(change) = next(&mut presence) => {
                if let Ok(present) = change.get().await {
                    apply(tx, BatteryEvent::Presence(presence_of(present)));
                }
            }
            Some(change) = next(&mut level) => {
                if let Ok(value) = change.get().await {
                    apply(tx, BatteryEvent::Level(percent_of(value)));
                }
            }
            Some(change) = next(&mut charge) => {
                if let Ok(value) = change.get().await {
                    apply(tx, BatteryEvent::Charge(charge_of(value)));
                }
            }
            Some(change) = next(&mut to_empty) => {
                if let Ok(value) = change.get().await {
                    apply(tx, BatteryEvent::TimeToEmpty(seconds_of(value)));
                }
            }
            Some(change) = next(&mut to_full) => {
                if let Ok(value) = change.get().await {
                    apply(tx, BatteryEvent::TimeToFull(seconds_of(value)));
                }
            }
            command = commands.recv() => match command {
                Some(BatteryCommand::Refresh) => {
                    let _ = device.refresh().await;
                }
                None => return Ended::Commands,
            }
        }
    }
}

/// Reads every property once and sends the resulting `Battery`.
async fn seed(tx: &watch::Sender<Battery>, device: &DeviceProxy<'_>) {
    if let Ok(present) = device.is_present().await {
        apply(tx, BatteryEvent::Presence(presence_of(present)));
    }
    if let Ok(value) = device.percentage().await {
        apply(tx, BatteryEvent::Level(percent_of(value)));
    }
    if let Ok(value) = device.state().await {
        apply(tx, BatteryEvent::Charge(charge_of(value)));
    }
    if let Ok(value) = device.time_to_empty().await {
        apply(tx, BatteryEvent::TimeToEmpty(seconds_of(value)));
    }
    if let Ok(value) = device.time_to_full().await {
        apply(tx, BatteryEvent::TimeToFull(seconds_of(value)));
    }
}

/// Reduces `event` onto the receiver's current value and publishes the result.
fn apply(tx: &watch::Sender<Battery>, event: BatteryEvent) {
    let next = step(tx.borrow().clone(), event);
    let _ = tx.send(next);
}

fn presence_of(is_present: bool) -> Presence {
    if is_present {
        Presence::Present
    } else {
        Presence::Absent
    }
}

fn percent_of(percentage: f64) -> Percent {
    Percent(percentage.round() as u16)
}

fn seconds_of(value: i64) -> Seconds {
    Seconds(value.max(0) as u64)
}

fn charge_of(state: BatteryState) -> Charge {
    Charge::from_upower(state as u32)
}
