//! `use_kbd_backlight`: UPower's keyboard backlight, absent on a machine without one.

use dioxus::prelude::ReadSignal;
use shell_host::use_watch;
use sill_services::KbdBacklight;

use super::use_services;

/// The keyboard backlight's step and range.
pub fn use_kbd_backlight() -> ReadSignal<KbdBacklight> {
    use_watch(use_services().kbd_backlight)
}
