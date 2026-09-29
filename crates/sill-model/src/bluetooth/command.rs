//! What the bluetooth service hears and what it can be told.

use serde::{Deserialize, Serialize};

use super::{BtAddress, BtBattery, BtConnection, BtDevice, BtPower};

/// One change BlueZ reported (or a command's own progress).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum BluetoothEvent {
    /// The adapter appeared, or its radio changed.
    Adapter { power: BtPower },
    /// A device appeared, or any of its properties changed: the whole device, replacing the one
    /// with its address.
    Device(BtDevice),
    /// A device's connection changed (including `Connecting` when a connect is sent).
    Connection {
        address: BtAddress,
        connection: BtConnection,
    },
    /// A device's battery changed or went away.
    Battery {
        address: BtAddress,
        battery: BtBattery,
    },
    /// BlueZ dropped a device.
    DeviceRemoved(BtAddress),
    /// The adapter went away, or BlueZ left the bus.
    Gone,
}

/// What the Bluetooth service can be told.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BluetoothCommand {
    /// Turn the adapter's radio on or off (`Adapter1.Powered`).
    Power(BtPower),
    /// Connect a paired device (`Device1.Connect`).
    Connect(BtAddress),
    /// Disconnect a device (`Device1.Disconnect`).
    Disconnect(BtAddress),
    /// Re-read the adapter and every device (after a resume).
    Refresh,
}
