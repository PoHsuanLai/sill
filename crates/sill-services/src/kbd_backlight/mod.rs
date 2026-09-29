//! The keyboard backlight: UPower's `org.freedesktop.UPower.KbdBacklight` over the system bus
//! (design/26 5.2.7: the control center's Keyboard Brightness module).
//!
//! UPower rather than `/sys/class/leds/*::kbd_backlight`: UPower finds the LED itself, lets the
//! session set it with no udev rule or group, and signals every change, the laptop's own
//! brightness keys included (`BrightnessChanged`), so the module follows the keys without a
//! poll. On a machine with no keyboard backlight UPower exports no such object (or answers its
//! `GetMaxBrightness` with 0 or an error), and the service stays absent, so the module hides.

mod backend;

use sill_model::kbd_backlight::{KbdBacklight, KbdCommand};

use crate::env::Env;
use crate::error::ServiceError;
use crate::service::{Service, ServiceIo};

/// The keyboard backlight as a [`Service`]. Without UPower or a backlight the state stays at
/// `KbdBacklight::default()` (absent), and the control center shows no module.
#[derive(Debug)]
pub enum KbdBacklightService {}

impl Service for KbdBacklightService {
    const NAME: &'static str = "kbd_backlight";
    type State = KbdBacklight;
    type Command = KbdCommand;
    type Deps = ();

    fn run(
        env: &Env,
        (): (),
        io: ServiceIo<Self>,
    ) -> impl Future<Output = Result<(), ServiceError>> + Send + 'static {
        backend::run(env.buses.system.clone(), io.state, io.commands)
    }
}
