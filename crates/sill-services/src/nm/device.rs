//! `org.freedesktop.NetworkManager.Device`, and the two enums sill reads from it.

use zbus::proxy;

#[proxy(
    interface = "org.freedesktop.NetworkManager.Device",
    default_service = "org.freedesktop.NetworkManager"
)]
pub(crate) trait Device {
    fn disconnect(&self) -> zbus::Result<()>;

    #[zbus(property)]
    fn device_type(&self) -> zbus::Result<u32>;

    #[zbus(property)]
    fn state(&self) -> zbus::Result<u32>;
}

/// `NMDeviceType`, as far as sill tells the kinds apart.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DeviceType {
    Wifi,
    Other,
}

impl From<u32> for DeviceType {
    fn from(value: u32) -> DeviceType {
        match value {
            2 => DeviceType::Wifi,
            _ => DeviceType::Other,
        }
    }
}

/// `NMDeviceState`, as far as sill tells the states apart.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum DeviceState {
    #[default]
    Unknown,
    Disconnected,
    Prepare,
    Config,
    NeedAuth,
    IpConfig,
    IpCheck,
    Secondaries,
    Activated,
    Other,
}

impl From<u32> for DeviceState {
    fn from(value: u32) -> DeviceState {
        match value {
            0 => DeviceState::Unknown,
            30 => DeviceState::Disconnected,
            40 => DeviceState::Prepare,
            50 => DeviceState::Config,
            60 => DeviceState::NeedAuth,
            70 => DeviceState::IpConfig,
            80 => DeviceState::IpCheck,
            90 => DeviceState::Secondaries,
            100 => DeviceState::Activated,
            _ => DeviceState::Other,
        }
    }
}
