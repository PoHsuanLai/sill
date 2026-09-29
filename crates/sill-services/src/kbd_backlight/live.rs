//! UPower's keyboard backlight over the system bus: seeds the step and range, then feeds every
//! `BrightnessChanged` through [`super::reduce_kbd_backlight`], and carries out
//! [`KbdCommand`]s. Only a `KbdCommand::Set` writes the backlight; tests never reach this file
//! (they feed a fake `watch` channel).

use tokio::sync::{mpsc, watch};

use super::KbdCommand;
use super::model::{KbdBacklight, KbdEvent, KbdLevel, reduce_kbd_backlight, step_for};
use super::upower::KbdBacklightProxy;
use crate::env::Bus;
use crate::stream::next_opt;
use crate::units::Presence;

/// Runs until `commands` closes. Without a bus or UPower, `tx` stays at its default.
pub(super) async fn run(
    bus: Bus,
    tx: watch::Sender<KbdBacklight>,
    mut commands: mpsc::UnboundedReceiver<KbdCommand>,
) {
    let Ok(Some(connection)) = bus.connect().await else {
        return;
    };
    let Ok(proxy) = KbdBacklightProxy::new(&connection).await else {
        return;
    };
    seed(&tx, &proxy).await;
    let mut changes = proxy.receive_brightness_changed().await.ok();

    loop {
        tokio::select! {
            Some(signal) = next_opt(&mut changes) => {
                if let Ok(args) = signal.args() {
                    apply(&tx, KbdEvent::Brightness(u32::try_from(args.value).unwrap_or(0)));
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
                Some(KbdCommand::Refresh) => seed(&tx, &proxy).await,
                None => break,
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
        .and_then(step)
        .filter(|max| *max > 0);
    let value = proxy.get_brightness().await.ok().and_then(step);
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
fn step(value: i32) -> Option<u32> {
    u32::try_from(value).ok()
}

/// Reduces `event` onto the receiver's current value and publishes the result.
fn apply(tx: &watch::Sender<KbdBacklight>, event: KbdEvent) {
    let next = reduce_kbd_backlight(*tx.borrow(), event);
    let _ = tx.send(next);
}
