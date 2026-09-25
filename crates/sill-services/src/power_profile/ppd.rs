//! Hand-written `zbus` proxies for power-profiles-daemon's two bus names (FINDINGS "M5 freeze"
//! F173). The same daemon owns both `net.hadess.PowerProfiles` (every release) and
//! `org.freedesktop.UPower.PowerProfiles` (0.20 on, what Fedora ships), each at its own object
//! path with an interface named after itself — not one interface reachable two ways, hence two
//! proxy traits rather than one parameterised by bus name.

use zbus::proxy;

#[proxy(
    interface = "org.freedesktop.UPower.PowerProfiles",
    default_service = "org.freedesktop.UPower.PowerProfiles",
    default_path = "/org/freedesktop/UPower/PowerProfiles"
)]
pub(super) trait UPowerPowerProfiles {
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

#[proxy(
    interface = "net.hadess.PowerProfiles",
    default_service = "net.hadess.PowerProfiles",
    default_path = "/net/hadess/PowerProfiles"
)]
pub(super) trait HadessPowerProfiles {
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
