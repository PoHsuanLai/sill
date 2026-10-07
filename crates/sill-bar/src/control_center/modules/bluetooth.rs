//! The Bluetooth tile: a quire `ModuleTile` whose press powers the adapter on or off,
//! [`crate::control_center::bluetooth_tile_status`] ("On", "Off") as its line, and its
//! chevron to the device list (`Detail::Bluetooth`). Reads `use_service::<BluetoothService>`; sends
//! `BluetoothCommand::Power`, asking first before off ([`super::bluetooth_ask`]).

use dioxus::prelude::*;
use ds::prelude::*;
use sill_model::bluetooth::{Bluetooth, BtPower};
use sill_model::units::Presence;
use sill_shell_kit::prelude::*;

use super::cc_module::{BarItemFace, CcCx, CcModule, Span};
use crate::bar::{BarMenu, StatusIcon};
use crate::control_center::modules::bluetooth_ask::use_power_request;
use crate::control_center::modules::flipped;
use crate::control_center::page::Detail;
use crate::control_center::pages::bluetooth_devices::BluetoothPane;
use crate::control_center::{bluetooth_status, bluetooth_tile_status};
use sill_services::BluetoothService;
use sill_settings::{ControlCenterSettings, InMenuBar};
use sill_ui_kit::hooks::use_service;
use sill_ui_kit::status::glyph::bluetooth_glyph_state;

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
            value: state,
            availability: adapter_availability(&bluetooth),
            onclick: move |_| request.call(power_of(flipped(state))),
            on_detail: move |_| on_detail.call(Detail::Bluetooth),
        }
    }
}

pub(crate) fn power_state(power: BtPower) -> Check {
    match power {
        BtPower::On => Check::On,
        BtPower::Off => Check::Off,
    }
}

pub(crate) fn power_of(state: Check) -> BtPower {
    match state {
        Check::On | Check::Mixed => BtPower::On,
        Check::Off => BtPower::Off,
    }
}

/// Disabled without an adapter.
pub(crate) fn adapter_availability(bluetooth: &Bluetooth) -> Availability {
    match bluetooth.presence {
        Presence::Present => Availability::Enabled,
        Presence::Absent => Availability::Disabled,
    }
}

/// The Bluetooth item in the bar: off, on, connecting, connected, or a failed connect.
#[component]
fn BluetoothItem() -> Element {
    let bluetooth = use_service::<BluetoothService>()();
    rsx! {
        StatusIcon {
            menu: BarMenu::Bluetooth,
            icon: StatusState::Bluetooth(bluetooth_glyph_state(&bluetooth)),
            tooltip: format!("Bluetooth: {}", bluetooth_status(&bluetooth)),
        }
    }
}

/// The Bluetooth module as the control center and the bar draw it.
pub(crate) struct BluetoothCc;

impl CcModule for BluetoothCc {
    fn span(&self) -> Span {
        Span::One
    }

    fn tile(&self, cx: &CcCx) -> Element {
        rsx! { BluetoothModule { on_detail: cx.on_detail } }
    }

    fn dropdown(&self, _cx: &CcCx) -> Option<Element> {
        Some(rsx! { BluetoothPane { back: None } })
    }

    fn menu_bar(&self, settings: &ControlCenterSettings) -> InMenuBar {
        settings.menu_bar_bluetooth
    }

    fn bar_item(&self) -> Option<BarItemFace> {
        Some(BarItemFace {
            menu: BarMenu::Bluetooth,
            item: || rsx! { BluetoothItem {} },
        })
    }

    fn detail(&self, page: Detail, cx: &CcCx) -> Option<Element> {
        (page == Detail::Bluetooth).then(|| rsx! { BluetoothPane { back: cx.on_back } })
    }
}

#[cfg(test)]
mod tests {
    use super::{adapter_availability, power_of, power_state};
    use ds::prelude::*;
    use sill_model::bluetooth::{Bluetooth, BtPower};
    use sill_model::units::Presence;

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
        assert_eq!(power_state(BtPower::On), Check::On);
        assert_eq!(power_of(Check::Off), BtPower::Off);
    }
}
