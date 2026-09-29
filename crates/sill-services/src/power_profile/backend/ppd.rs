//! A hand-written `zbus` proxy for power-profiles-daemon's `org.freedesktop.UPower.PowerProfiles`
//! (0.20 or later).

use zbus::proxy;

#[proxy(
    interface = "org.freedesktop.UPower.PowerProfiles",
    default_service = "org.freedesktop.UPower.PowerProfiles",
    default_path = "/org/freedesktop/UPower/PowerProfiles"
)]
pub(crate) trait UPowerPowerProfiles {
    #[zbus(property)]
    fn active_profile(&self) -> zbus::Result<String>;

    #[zbus(property)]
    fn set_active_profile(&self, value: &str) -> zbus::Result<()>;

    #[zbus(property)]
    fn profiles(
        &self,
    ) -> zbus::Result<Vec<std::collections::HashMap<String, zbus::zvariant::OwnedValue>>>;

    #[zbus(property)]
    fn performance_degraded(&self) -> zbus::Result<String>;
}
