//! The Bluetooth tile: the adapter's radio as a quire `Toggle`,
//! [`crate::surfaces::control_center::bluetooth_status`] as its line, and a chevron to the
//! device list (`Detail::Bluetooth`). Reads `use_bluetooth`; sends `BluetoothCommand::Power`.

use dioxus::prelude::*;
use ds::{Availability, Icon};
use sill_services::{Bluetooth, BluetoothCommand, BtConnection, BtPower, Presence};

use crate::hooks::{use_bluetooth, use_services};
use crate::surfaces::control_center::bluetooth_status;
use crate::surfaces::control_center::page::Detail;
use crate::surfaces::control_center::parts::{ModuleState, ModuleTile};

/// The Bluetooth tile.
#[component]
pub fn BluetoothModule(on_detail: EventHandler<Detail>) -> Element {
    let bluetooth = use_bluetooth()();
    let control = use_services().bluetooth_control;
    rsx! {
        ModuleTile {
            glyph: bluetooth_glyph(&bluetooth),
            title: "Bluetooth".to_owned(),
            status: bluetooth_status(&bluetooth),
            state: power_state(bluetooth.power),
            availability: adapter_availability(&bluetooth),
            on_toggle: move |to: ModuleState| {
                // A stopped service drops it: the tile shows the power it last published.
                let _gone = control.send(BluetoothCommand::Power(power_of(to)));
            },
            on_detail: move |()| on_detail.call(Detail::Bluetooth),
        }
    }
}

pub(crate) fn power_state(power: BtPower) -> ModuleState {
    match power {
        BtPower::On => ModuleState::On,
        BtPower::Off => ModuleState::Off,
    }
}

pub(crate) fn power_of(state: ModuleState) -> BtPower {
    match state {
        ModuleState::On => BtPower::On,
        ModuleState::Off => BtPower::Off,
    }
}

/// Disabled without an adapter.
pub(crate) fn adapter_availability(bluetooth: &Bluetooth) -> Availability {
    match bluetooth.presence {
        Presence::Present => Availability::Enabled,
        Presence::Absent => Availability::Disabled,
    }
}

/// Off, on, or on with a device connected.
pub(crate) fn bluetooth_glyph(bluetooth: &Bluetooth) -> Icon {
    let connected = bluetooth
        .devices
        .iter()
        .any(|d| d.connection == BtConnection::Connected);
    match (bluetooth.presence, bluetooth.power, connected) {
        (Presence::Absent, _, _) | (_, BtPower::Off, _) => Icon::BluetoothOff,
        (Presence::Present, BtPower::On, true) => Icon::BluetoothConnected,
        (Presence::Present, BtPower::On, false) => Icon::Bluetooth,
    }
}

#[cfg(test)]
mod tests {
    use super::{adapter_availability, bluetooth_glyph, power_of, power_state};
    use crate::surfaces::control_center::parts::ModuleState;
    use ds::{Availability, Icon};
    use sill_services::{
        Bluetooth, BtAddress, BtBattery, BtConnection, BtDevice, BtKind, BtPairing, BtPower,
        Presence,
    };

    #[test]
    fn the_tile_follows_the_adapter() {
        let buds = BtDevice {
            address: BtAddress("AA".to_owned()),
            name: "Buds".to_owned(),
            kind: BtKind::Headphones,
            connection: BtConnection::Connected,
            pairing: BtPairing::Paired,
            battery: BtBattery::Unknown,
        };
        let on = |devices| Bluetooth {
            presence: Presence::Present,
            power: BtPower::On,
            devices,
        };
        assert_eq!(bluetooth_glyph(&Bluetooth::default()), Icon::BluetoothOff);
        assert_eq!(bluetooth_glyph(&on(Vec::new())), Icon::Bluetooth);
        assert_eq!(bluetooth_glyph(&on(vec![buds])), Icon::BluetoothConnected);
        assert_eq!(
            adapter_availability(&Bluetooth::default()),
            Availability::Disabled
        );
        assert_eq!(adapter_availability(&on(Vec::new())), Availability::Enabled);
        assert_eq!(power_state(BtPower::On), ModuleState::On);
        assert_eq!(power_of(ModuleState::Off), BtPower::Off);
    }
}
