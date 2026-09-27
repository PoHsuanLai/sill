//! A hand-written `zbus` proxy for UPower's keyboard backlight (FINDINGS "M5 freeze" F173: no
//! crate for three methods and a signal).

use zbus::proxy;

#[proxy(
    interface = "org.freedesktop.UPower.KbdBacklight",
    default_service = "org.freedesktop.UPower",
    default_path = "/org/freedesktop/UPower/KbdBacklight"
)]
pub(super) trait KbdBacklight {
    fn get_brightness(&self) -> zbus::Result<i32>;
    fn get_max_brightness(&self) -> zbus::Result<i32>;
    fn set_brightness(&self, value: i32) -> zbus::Result<()>;

    /// Every change, whoever made it.
    #[zbus(signal)]
    fn brightness_changed(&self, value: i32) -> zbus::Result<()>;
}
