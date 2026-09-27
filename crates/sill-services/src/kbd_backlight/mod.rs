//! The keyboard backlight: UPower's `org.freedesktop.UPower.KbdBacklight` over the system bus
//! (design/26 5.2.7 G25: the control center's Keyboard Brightness module).
//!
//! UPower rather than `/sys/class/leds/*::kbd_backlight`: UPower finds the LED itself, lets the
//! session set it with no udev rule or group, and signals every change, the laptop's own
//! brightness keys included (`BrightnessChanged`), so the module follows the keys without a
//! poll. On a machine with no keyboard backlight UPower exports no such object (or answers its
//! `GetMaxBrightness` with 0 or an error), and the service stays absent, so the module hides.

mod live;
mod model;
mod upower;

pub use model::{KbdBacklight, KbdEvent, KbdLevel, reduce_kbd_backlight, step_for};

use tokio::runtime::Handle;
use tokio::sync::watch;

use crate::control::Control;
use crate::units::Percent;

/// What the keyboard backlight service can be told.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KbdCommand {
    /// Set the backlight to the step nearest `Percent` of its range; dropped while absent.
    Set(Percent),
    /// Re-read the backlight (after a resume).
    Refresh,
}

/// Start the keyboard backlight task on `rt`. Without UPower or a backlight the receiver stays
/// at `KbdBacklight::default()` (absent), and the control center shows no module.
pub fn spawn(rt: &Handle) -> (watch::Receiver<KbdBacklight>, Control<KbdCommand>) {
    let (tx, rx) = watch::channel(KbdBacklight::default());
    let (control, commands) = Control::channel();
    rt.spawn(live::run(tx, commands));
    (rx, control)
}
