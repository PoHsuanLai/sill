//! The Bluetooth tile: a quire `ModuleTile` whose press powers the adapter on or off,
//! [`crate::surfaces::control_center::bluetooth_tile_status`] ("On", "Off") as its line, and its
//! chevron to the device list (`Detail::Bluetooth`). Reads `use_bluetooth`; sends `BluetoothCommand::Power`.

use dioxus::prelude::*;
use ds::{Availability, Chevron, ModuleState, ModuleTile, StatusState, Text};
use sill_services::{Bluetooth, BluetoothCommand, BtPower, Presence};

use crate::hooks::{use_bluetooth, use_services};
use crate::surfaces::bar::glyph_state::bluetooth_glyph_state;
use crate::surfaces::control_center::bluetooth_tile_status;
use crate::surfaces::control_center::modules::flipped;
use crate::surfaces::control_center::page::Detail;

/// The Bluetooth tile.
#[component]
pub fn BluetoothModule(on_detail: EventHandler<Detail>) -> Element {
    let bluetooth = use_bluetooth()();
    let control = use_services().bluetooth_control;
    let state = power_state(bluetooth.power);
    rsx! {
        ModuleTile {
            glyph: StatusState::Bluetooth(bluetooth_glyph_state(&bluetooth)),
            title: "Bluetooth",
            status: Some(Text::from(bluetooth_tile_status(&bluetooth))),
            state,
            chevron: Chevron::Detail,
            availability: adapter_availability(&bluetooth),
            onclick: move |_| {
                // A stopped service drops it: the tile shows the power it last published.
                let _gone = control.send(BluetoothCommand::Power(power_of(flipped(state))));
            },
            on_detail: move |_| on_detail.call(Detail::Bluetooth),
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
        ModuleState::On | ModuleState::Busy => BtPower::On,
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

#[cfg(test)]
mod tests {
    use super::{adapter_availability, power_of, power_state};
    use ds::{Availability, ModuleState};
    use sill_services::{Bluetooth, BtPower, Presence};

    #[test]
    fn the_tile_follows_the_adapter() {
        let on = Bluetooth {
            presence: Presence::Present,
            power: BtPower::On,
            devices: Vec::new(),
            connect: Default::default(),
        };
        assert_eq!(
            adapter_availability(&Bluetooth::default()),
            Availability::Disabled
        );
        assert_eq!(adapter_availability(&on), Availability::Enabled);
        assert_eq!(power_state(BtPower::On), ModuleState::On);
        assert_eq!(power_of(ModuleState::Off), BtPower::Off);
    }
}
