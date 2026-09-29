//! The Bluetooth tile: a quire `ModuleTile` whose press powers the adapter on or off,
//! [`crate::surfaces::control_center::bluetooth_tile_status`] ("On", "Off") as its line, and its
//! chevron to the device list (`Detail::Bluetooth`). Reads `use_service::<BluetoothService>`; sends
//! `BluetoothCommand::Power`, asking first before off ([`super::bluetooth_ask`]).

use dioxus::prelude::*;
use ds::{Availability, Chevron, ModuleState, ModuleTile, StatusState, TextLine};
use sill_model::units::Presence;
use sill_services::{Bluetooth, BtPower};

use crate::hooks::use_service;
use crate::surfaces::bar::glyph_state::bluetooth_glyph_state;
use crate::surfaces::control_center::bluetooth_tile_status;
use crate::surfaces::control_center::modules::bluetooth_ask::use_power_request;
use crate::surfaces::control_center::modules::flipped;
use crate::surfaces::control_center::page::Detail;
use sill_services::BluetoothService;

/// The Bluetooth tile.
#[component]
pub fn BluetoothModule(on_detail: EventHandler<Detail>) -> Element {
    let bluetooth = use_service::<BluetoothService>()();
    let request = use_power_request();
    let state = power_state(bluetooth.power);
    rsx! {
        ModuleTile {
            glyph: StatusState::Bluetooth(bluetooth_glyph_state(&bluetooth)),
            title: "Bluetooth",
            status: Some(TextLine::from(bluetooth_tile_status(&bluetooth))),
            state,
            chevron: Chevron::Detail,
            availability: adapter_availability(&bluetooth),
            onclick: move |_| request.call(power_of(flipped(state))),
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
    use sill_model::units::Presence;
    use sill_services::{Bluetooth, BtPower};

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
