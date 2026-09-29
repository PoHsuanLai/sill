//! The Bluetooth page: the radio `Toggle` in its header, then `listed_devices` as rows (kind
//! glyph on a disc, the accent disc when connected, name,
//! [`crate::surfaces::control_center::device_detail`], a connected device's battery at the
//! end); choosing a paired device sends `BluetoothCommand::Connect` or `Disconnect`. The
//! connect's stamp (`Bluetooth.connect`) is the connecting row's phase: its glyph breathes, its
//! disc seals when it connects, it shakes when it fails
//! ([`crate::surfaces::control_center::row_phase`]). Pairing a new device is COSMIC Settings'
//! ("Bluetooth Settings…" at the foot).

use dioxus::prelude::*;
use ds::{Availability, Icon, ModuleState, RowDisc, RowPhase, RowWork};
use sill_model::attempt::Attempts;
use sill_model::bluetooth::{
    Bluetooth, BluetoothCommand, BtAddress, BtBattery, BtConnection, BtDevice, BtKind, BtPairing,
    BtPower, listed_devices,
};
use sill_model::units::Presence;
use sill_services::BluetoothService;

use super::foot::{EmptyLine, SettingsFoot};
use super::head::DetailHead;
use crate::hooks::{use_control, use_service};
use crate::surfaces::control_center::device_detail;
use crate::surfaces::control_center::modules::bluetooth::{
    adapter_availability, power_of, power_state,
};
use crate::surfaces::control_center::modules::bluetooth_ask::use_power_request;
use crate::surfaces::control_center::row_phase::{Aim, FAILED_WORDS, Item, aim_after, row_phase};
use crate::surfaces::control_center::rows::{RowList, RowMark, RowSpec, use_first_show};

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
    let bluetooth = use_service::<BluetoothService>()();
    let control = use_control::<BluetoothService>();
    let first = use_first_show();
    let mut aim = use_hook(|| CopyValue::new(None::<BtAddress>));
    let last = aim_after(aim.peek().clone(), connecting(&bluetooth));
    aim.set(last.clone());
    let rows = device_rows(&bluetooth, last.as_ref());
    let empty = empty_line(&bluetooth, rows.len());
    let request = use_power_request();
    rsx! {
        div { class: "sill-cc-detail",
            DetailHead {
                title: "Bluetooth".to_owned(),
                back,
                state: power_state(bluetooth.power),
                availability: adapter_availability(&bluetooth),
                // Off asks first (`bluetooth_ask`); the switch shows the power last published.
                on_toggle: move |to: ModuleState| request.call(power_of(to)),
            }
            if let Some(text) = empty {
                EmptyLine { text }
            } else {
                RowList::<BluetoothCommand> {
                    label: "Devices".to_owned(),
                    rows,
                    first,
                    onpick: move |command: BluetoothCommand| {
                        let _gone = control.send(command);
                    },
                }
            }
            SettingsFoot { label: "Bluetooth Settings…".to_owned() }
        }
    }
}

/// The device connecting now, if any.
pub(crate) fn connecting(bluetooth: &Bluetooth) -> Option<BtAddress> {
    bluetooth
        .devices
        .iter()
        .find(|device| device.connection == BtConnection::Connecting)
        .map(|device| device.address.clone())
}

/// The list's rows while the adapter is on (`listed_devices` orders them), each in its phase of
/// the connect attempts; `last` is the device the pane last saw connecting.
pub(crate) fn device_rows(
    bluetooth: &Bluetooth,
    last: Option<&BtAddress>,
) -> Vec<RowSpec<BluetoothCommand>> {
    match (bluetooth.presence, bluetooth.power) {
        (Presence::Present, BtPower::On) => listed_devices(bluetooth)
            .into_iter()
            .map(|device| {
                let aim = if last == Some(&device.address) {
                    Aim::Last
                } else {
                    Aim::Other
                };
                device_row(device, phase_of(bluetooth.connect, device, aim))
            })
            .collect(),
        (Presence::Absent, _) | (_, BtPower::Off) => Vec::new(),
    }
}

/// `device`'s phase in the connect attempts.
fn phase_of(connect: Attempts, device: &BtDevice, aim: Aim) -> RowPhase {
    let item = match device.connection {
        BtConnection::Disconnected => Item::Idle,
        BtConnection::Connecting => Item::Connecting,
        BtConnection::Connected => Item::InUse,
    };
    row_phase(connect, item, aim)
}

/// One device's row in `phase`: a connected one disconnects, a paired one connects, a stranger
/// (paired in COSMIC Settings) is shown and cannot be chosen. The accent disc and the battery
/// at the end say it is connected (no check); the failure's words go in the detail line.
pub(crate) fn device_row(device: &BtDevice, phase: RowPhase) -> RowSpec<BluetoothCommand> {
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
    let (mark, disc) = match (device.connection, device.battery) {
        (BtConnection::Connected, BtBattery::Level(level)) => {
            (RowMark::Battery(level), RowDisc::On)
        }
        (BtConnection::Connected, BtBattery::Unknown) => (RowMark::Nothing, RowDisc::On),
        (BtConnection::Connecting | BtConnection::Disconnected, _) => {
            (RowMark::Nothing, RowDisc::Off)
        }
    };
    let detail = match phase {
        RowPhase::Failed(_) => FAILED_WORDS.to_owned(),
        RowPhase::Rest | RowPhase::Pending(_) | RowPhase::Succeeded(_) => device_detail(device),
    };
    RowSpec {
        key: device.address.0.clone(),
        value,
        glyph: device_glyph(device.kind),
        title: device.name.clone(),
        detail,
        mark,
        phase,
        work: RowWork::Glyph,
        disc,
        availability,
    }
}

/// A device's glyph by kind.
pub(crate) fn device_glyph(kind: BtKind) -> Icon {
    match kind {
        BtKind::Headphones | BtKind::Headset => Icon::Headphones,
        BtKind::Speaker => Icon::Speaker,
        BtKind::Keyboard => Icon::Keyboard,
        BtKind::Mouse => Icon::Mouse,
        BtKind::Gamepad => Icon::Gamepad,
        BtKind::Phone => Icon::Phone,
        BtKind::Computer => Icon::Monitor,
        BtKind::Other => Icon::Bluetooth,
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
    use super::{connecting, device_row, device_rows, empty_line};
    use crate::surfaces::control_center::rows::RowMark;
    use ds::detail::EventStamp;
    use ds::{Availability, RowDisc, RowPhase};
    use sill_model::bluetooth::{
        Bluetooth, BluetoothCommand, BtAddress, BtBattery, BtConnection, BtDevice, BtKind,
        BtPairing, BtPower,
    };
    use sill_model::{
        attempt::{Attempt, Attempts, OpStamp},
        units::{Percent, Presence},
    };

    fn device(connection: BtConnection, pairing: BtPairing, battery: BtBattery) -> BtDevice {
        BtDevice {
            address: BtAddress("AA".to_owned()),
            name: "Buds".to_owned(),
            kind: BtKind::Headphones,
            connection,
            pairing,
            battery,
        }
    }

    #[test]
    fn each_device_row_connects_or_disconnects() {
        let aa = || BtAddress("AA".to_owned());
        let level = BtBattery::Level(Percent(84));
        // (connection, pairing, battery, command, availability, mark, disc)
        let cases = [
            (
                BtConnection::Connected,
                BtPairing::Paired,
                level,
                BluetoothCommand::Disconnect(aa()),
                Availability::Enabled,
                RowMark::Battery(Percent(84)),
                RowDisc::On,
            ),
            (
                BtConnection::Connected,
                BtPairing::Paired,
                BtBattery::Unknown,
                BluetoothCommand::Disconnect(aa()),
                Availability::Enabled,
                RowMark::Nothing,
                RowDisc::On,
            ),
            (
                BtConnection::Disconnected,
                BtPairing::Paired,
                level,
                BluetoothCommand::Connect(aa()),
                Availability::Enabled,
                RowMark::Nothing,
                RowDisc::Off,
            ),
            (
                BtConnection::Disconnected,
                BtPairing::Unpaired,
                BtBattery::Unknown,
                BluetoothCommand::Connect(aa()),
                Availability::Disabled,
                RowMark::Nothing,
                RowDisc::Off,
            ),
        ];
        for (connection, pairing, battery, command, availability, mark, disc) in cases {
            let row = device_row(&device(connection, pairing, battery), RowPhase::Rest);
            assert_eq!(
                (row.value, row.availability, row.mark, row.disc),
                (command, availability, mark, disc),
                "{connection:?} {pairing:?} {battery:?}"
            );
        }
    }

    #[test]
    fn the_connecting_row_takes_the_stamp_then_fails_with_the_new_one() {
        let aa = BtAddress("AA".to_owned());
        let mut bluetooth = Bluetooth {
            presence: Presence::Present,
            power: BtPower::On,
            devices: vec![device(
                BtConnection::Connecting,
                BtPairing::Paired,
                BtBattery::Unknown,
            )],
            connect: Attempts {
                now: Attempt::Running(OpStamp(3)),
                minted: OpStamp(3),
            },
        };
        assert_eq!(connecting(&bluetooth), Some(aa.clone()));
        let phase = |bluetooth: &Bluetooth| device_rows(bluetooth, Some(&aa))[0].phase;
        assert_eq!(phase(&bluetooth), RowPhase::Pending(EventStamp(3)));
        bluetooth.devices[0].connection = BtConnection::Disconnected;
        bluetooth.connect = Attempts {
            now: Attempt::Failed(OpStamp(4)),
            minted: OpStamp(4),
        };
        let row = &device_rows(&bluetooth, Some(&aa))[0];
        assert_eq!(
            (row.phase, row.detail.as_str()),
            (RowPhase::Failed(EventStamp(4)), "Couldn’t connect")
        );
        assert_eq!(
            device_rows(&bluetooth, None)[0].phase,
            RowPhase::Rest,
            "a failure the pane never saw start"
        );
    }

    #[test]
    fn the_pane_says_why_there_is_no_list() {
        let on = Bluetooth {
            presence: Presence::Present,
            power: BtPower::On,
            devices: Vec::new(),
            connect: Default::default(),
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
