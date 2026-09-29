//! UPower's display device over the system bus: seeds a real first value, then feeds every
//! property change through [`step`].

use tokio::sync::{mpsc, watch};
use upower_dbus::{BatteryState, DeviceProxy, UPowerProxy};

use super::BatteryCommand;
use super::model::{Battery, BatteryEvent, Charge, step};
use crate::env::Bus;
use crate::error::ServiceError;
use crate::stream::next;
use crate::units::{Percent, Presence, Seconds};

/// Connects to UPower's display device and runs until `commands` closes (the `Control` was
/// dropped). If the bus or UPower is unreachable, `tx` is left at its seeded default (absent).
pub(super) async fn run(
    bus: Bus,
    tx: watch::Sender<Battery>,
    mut commands: mpsc::UnboundedReceiver<BatteryCommand>,
) -> Result<(), ServiceError> {
    let connection = bus.connected().await?;
    let upower = UPowerProxy::new(&connection).await?;
    let device = upower.get_display_device().await?;

    seed(&tx, &device).await;

    let mut presence = device.receive_is_present_changed().await;
    let mut level = device.receive_percentage_changed().await;
    let mut charge = device.receive_state_changed().await;
    let mut to_empty = device.receive_time_to_empty_changed().await;
    let mut to_full = device.receive_time_to_full_changed().await;

    loop {
        tokio::select! {
            Some(change) = next(&mut presence) => {
                if let Ok(present) = change.get().await {
                    apply(&tx, BatteryEvent::Presence(presence_of(present)));
                }
            }
            Some(change) = next(&mut level) => {
                if let Ok(value) = change.get().await {
                    apply(&tx, BatteryEvent::Level(percent_of(value)));
                }
            }
            Some(change) = next(&mut charge) => {
                if let Ok(value) = change.get().await {
                    apply(&tx, BatteryEvent::Charge(charge_of(value)));
                }
            }
            Some(change) = next(&mut to_empty) => {
                if let Ok(value) = change.get().await {
                    apply(&tx, BatteryEvent::TimeToEmpty(seconds_of(value)));
                }
            }
            Some(change) = next(&mut to_full) => {
                if let Ok(value) = change.get().await {
                    apply(&tx, BatteryEvent::TimeToFull(seconds_of(value)));
                }
            }
            command = commands.recv() => match command {
                Some(BatteryCommand::Refresh) => {
                    let _ = device.refresh().await;
                }
                None => return Ok(()),
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
