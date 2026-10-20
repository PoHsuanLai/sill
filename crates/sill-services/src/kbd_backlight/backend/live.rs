//! UPower's keyboard backlight over the system bus: seeds the step and range, then feeds every
//! `BrightnessChanged` through [`sill_model::kbd_backlight::step`], and carries out
//! [`KbdCommand`]s. Only a `KbdCommand::Set` writes the backlight; tests never reach this file
//! (they feed a fake `watch` channel).

use sill_model::kbd_backlight::{KbdBacklight, KbdCommand, KbdEvent, KbdLevel, step, step_for};
use sill_model::units::Presence;
use tokio::sync::{mpsc, watch};
use zbus::Connection;

use super::upower::KbdBacklightProxy;
use crate::env::Bus;
use crate::error::ServiceError;
use crate::owner::{self, Ended};
use crate::stream::next_opt;

const SERVICE: &str = "org.freedesktop.UPower";

/// Runs until `commands` closes. Without a bus or UPower, `tx` stays at its default; UPower
/// starting late or restarting is followed through `NameOwnerChanged` ([`crate::owner`]).
pub(crate) async fn run(
    bus: Bus,
    tx: watch::Sender<KbdBacklight>,
    mut commands: mpsc::UnboundedReceiver<KbdCommand>,
) -> Result<(), ServiceError> {
    let connection = crate::supervise::connect_when_up(&bus).await?;
    owner::follow_owner(
        &connection,
        SERVICE,
        &mut commands,
        async |commands| session(&connection, &tx, commands).await,
        |_| apply(&tx, KbdEvent::Gone),
    )
    .await;
    Ok(())
}

/// One UPower lifetime: seed, then follow every brightness change and carry out commands.
async fn session(
    connection: &Connection,
    tx: &watch::Sender<KbdBacklight>,
    commands: &mut mpsc::UnboundedReceiver<KbdCommand>,
) -> Ended {
    let Ok(proxy) = KbdBacklightProxy::new(connection).await else {
        return Ended::Unreadable;
    };
    seed(tx, &proxy).await;
    let mut changes = proxy.receive_brightness_changed().await.ok();

    loop {
        tokio::select! {
            Some(signal) = next_opt(&mut changes) => {
                if let Ok(args) = signal.args() {
                    apply(tx, KbdEvent::Brightness(u32::try_from(args.value).unwrap_or(0)));
                }
            }
            command = commands.recv() => match command {
                Some(KbdCommand::Set(percent)) => {
                    let now = *tx.borrow();
                    if now.presence == Presence::Present {
                        let step = step_for(percent, now.level.max);
                        let _ = proxy.set_brightness(i32::try_from(step).unwrap_or(i32::MAX)).await;
                    }
                }
                Some(KbdCommand::Refresh) => seed(tx, &proxy).await,
                None => return Ended::Commands,
            }
        }
    }
}

/// Reads the range and the step once and publishes them; a range of 0 or an error is no
/// backlight.
async fn seed(tx: &watch::Sender<KbdBacklight>, proxy: &KbdBacklightProxy<'_>) {
    let max = proxy
        .get_max_brightness()
        .await
        .ok()
        .and_then(level_of)
        .filter(|max| *max > 0);
    let value = proxy.get_brightness().await.ok().and_then(level_of);
    let event = match max {
        Some(max) => KbdEvent::Found(KbdLevel {
            value: value.unwrap_or(0),
            max,
        }),
        None => KbdEvent::Gone,
    };
    apply(tx, event);
}

/// UPower's `i32` as a step: a negative one is an error.
fn level_of(value: i32) -> Option<u32> {
    u32::try_from(value).ok()
}

/// Reduces `event` onto the receiver's current value and publishes the result.
fn apply(tx: &watch::Sender<KbdBacklight>, event: KbdEvent) {
    let next = step(*tx.borrow(), event);
    let _ = tx.send(next);
}
