//! sill's own `zbus` proxies for NetworkManager's D-Bus API (`org.freedesktop.NetworkManager`),
//! one file per interface and only the members sill reads or calls. Written from the
//! documented API, so nothing here depends on a third-party binding crate.

mod access_point;
mod active;
mod device;
mod manager;
mod settings;
mod wireless;

pub(crate) use access_point::AccessPointProxy;
pub(crate) use active::ActiveConnectionProxy;
pub(crate) use device::{DeviceProxy, DeviceState, DeviceType};
pub(crate) use manager::NetworkManagerProxy;
pub(crate) use settings::{ConnectionSettingsProxy, SettingsProxy};
pub(crate) use wireless::WirelessDeviceProxy;
