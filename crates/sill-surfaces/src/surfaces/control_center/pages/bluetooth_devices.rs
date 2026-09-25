//! The Bluetooth page: the radio `Toggle` in its header, then `listed_devices` as rows (kind
//! glyph, name, [`crate::surfaces::control_center::device_detail`]); choosing a paired device
//! sends `BluetoothCommand::Connect` or `Disconnect`. Pairing a new device is COSMIC Settings'
//! ("Bluetooth Settings…" at the foot).

use dioxus::prelude::*;
use ds::{Availability, Check};
use sill_services::{
    Bluetooth, BluetoothCommand, BtAddress, BtConnection, BtDevice, BtPairing, BtPower, Presence,
    listed_devices,
};

use super::foot::{EmptyLine, SettingsFoot};
use super::head::DetailHead;
use crate::hooks::{use_bluetooth, use_services};
use crate::surfaces::control_center::device_detail;
use crate::surfaces::control_center::modules::bluetooth::{
    adapter_availability, power_of, power_state,
};
use crate::surfaces::control_center::parts::glyphs::device_glyph;
use crate::surfaces::control_center::parts::{ModuleState, RowMark, SettingsRow, SettingsRows};

/// The Bluetooth page.
#[component]
pub fn BluetoothDevices(on_back: EventHandler<()>) -> Element {
    rsx! {
        BluetoothPane { back: Some(on_back) }
    }
}

/// The Bluetooth pane, with a back button in the control center and without one as the
/// Bluetooth bar item's own dropdown.
#[component]
pub(crate) fn BluetoothPane(back: Option<EventHandler<()>>) -> Element {
    let bluetooth = use_bluetooth()();
    let control = use_services().bluetooth_control;
    let rows = device_rows(&bluetooth);
    let empty = empty_line(&bluetooth, rows.len());
    let toggle = control.clone();
    rsx! {
        div { class: "sill-cc-detail",
            DetailHead {
                title: "Bluetooth".to_owned(),
                back,
                state: power_state(bluetooth.power),
                availability: adapter_availability(&bluetooth),
                on_toggle: move |to: ModuleState| {
                    // A stopped service drops it: the switch shows the power it last published.
                    let _gone = toggle.send(BluetoothCommand::Power(power_of(to)));
                },
            }
            if let Some(text) = empty {
                EmptyLine { text }
            } else {
                SettingsRows::<BluetoothCommand> {
                    label: "Devices".to_owned(),
                    rows,
                    onpick: move |command: BluetoothCommand| {
                        let _gone = control.send(command);
                    },
                }
            }
            SettingsFoot { label: "Bluetooth Settings…".to_owned() }
        }
    }
}

/// The list's rows while the adapter is on (`listed_devices` orders them).
pub(crate) fn device_rows(bluetooth: &Bluetooth) -> Vec<SettingsRow<BluetoothCommand>> {
    match (bluetooth.presence, bluetooth.power) {
        (Presence::Present, BtPower::On) => listed_devices(bluetooth)
            .into_iter()
            .map(device_row)
            .collect(),
        (Presence::Absent, _) | (_, BtPower::Off) => Vec::new(),
    }
}

/// One device's row: a connected one disconnects, a paired one connects, a stranger (paired in
/// COSMIC Settings) is shown and cannot be chosen.
pub(crate) fn device_row(device: &BtDevice) -> SettingsRow<BluetoothCommand> {
    let address: BtAddress = device.address.clone();
    let (value, availability) = match (device.connection, device.pairing) {
        (BtConnection::Connected | BtConnection::Connecting, _) => {
            (BluetoothCommand::Disconnect(address), Availability::Enabled)
        }
        (BtConnection::Disconnected, BtPairing::Paired) => {
            (BluetoothCommand::Connect(address), Availability::Enabled)
        }
        (BtConnection::Disconnected, BtPairing::Unpaired) => {
            (BluetoothCommand::Connect(address), Availability::Disabled)
        }
    };
    let check = match device.connection {
        BtConnection::Connected => Check::Checked,
        BtConnection::Connecting | BtConnection::Disconnected => Check::Unchecked,
    };
    SettingsRow {
        value,
        glyph: device_glyph(device.kind),
        title: device.name.clone(),
        detail: device_detail(device),
        check,
        mark: RowMark::Nothing,
        availability,
    }
}

/// What the pane says instead of a list, if anything.
pub(crate) fn empty_line(bluetooth: &Bluetooth, rows: usize) -> Option<String> {
    let text = match (bluetooth.presence, bluetooth.power, rows) {
        (Presence::Absent, _, _) => "No Bluetooth adapter",
        (_, BtPower::Off, _) => "Bluetooth is off",
        (_, BtPower::On, 0) => "No devices",
        (_, BtPower::On, _) => return None,
    };
    Some(text.to_owned())
}

#[cfg(test)]
mod tests {
    use super::{device_row, empty_line};
    use ds::{Availability, Check};
    use sill_services::{
        Bluetooth, BluetoothCommand, BtAddress, BtBattery, BtConnection, BtDevice, BtKind,
        BtPairing, BtPower, Presence,
    };

    fn device(connection: BtConnection, pairing: BtPairing) -> BtDevice {
        BtDevice {
            address: BtAddress("AA".to_owned()),
            name: "Buds".to_owned(),
            kind: BtKind::Headphones,
            connection,
            pairing,
            battery: BtBattery::Unknown,
        }
    }

    #[test]
    fn each_device_row_connects_or_disconnects() {
        let aa = || BtAddress("AA".to_owned());
        let cases = [
            (
                BtConnection::Connected,
                BtPairing::Paired,
                BluetoothCommand::Disconnect(aa()),
                Availability::Enabled,
                Check::Checked,
            ),
            (
                BtConnection::Disconnected,
                BtPairing::Paired,
                BluetoothCommand::Connect(aa()),
                Availability::Enabled,
                Check::Unchecked,
            ),
            (
                BtConnection::Disconnected,
                BtPairing::Unpaired,
                BluetoothCommand::Connect(aa()),
                Availability::Disabled,
                Check::Unchecked,
            ),
        ];
        for (connection, pairing, command, availability, check) in cases {
            let row = device_row(&device(connection, pairing));
            assert_eq!(
                (row.value, row.availability, row.check),
                (command, availability, check),
                "{connection:?} {pairing:?}"
            );
        }
    }

    #[test]
    fn the_pane_says_why_there_is_no_list() {
        let on = Bluetooth {
            presence: Presence::Present,
            power: BtPower::On,
            devices: Vec::new(),
        };
        assert_eq!(
            empty_line(&Bluetooth::default(), 0).as_deref(),
            Some("No Bluetooth adapter")
        );
        assert_eq!(empty_line(&on, 0).as_deref(), Some("No devices"));
        assert_eq!(empty_line(&on, 2), None);
        let off = Bluetooth {
            power: BtPower::Off,
            ..on
        };
        assert_eq!(empty_line(&off, 0).as_deref(), Some("Bluetooth is off"));
    }
}
