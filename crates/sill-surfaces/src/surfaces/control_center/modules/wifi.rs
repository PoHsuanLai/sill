//! The Wi-Fi tile: a quire `ModuleTile` whose press turns the radio on or off,
//! [`crate::surfaces::control_center::wifi_status`] as its line, and its chevron to the network
//! list (`Detail::Wifi`). Reads `use_wifi` and
//! `use_network`; sends `WifiCommand::Radio`.

use dioxus::prelude::*;
use ds::{Availability, Chevron, Icon, ModuleState, ModuleTile, Text};
use sill_services::{Link, Network, Presence, Radio, Wifi, WifiCommand};

use crate::hooks::{use_network, use_services, use_wifi};
use crate::surfaces::bar::status::network_icon;
use crate::surfaces::control_center::modules::flipped;
use crate::surfaces::control_center::page::Detail;
use crate::surfaces::control_center::wifi_status;

/// The Wi-Fi tile.
#[component]
pub fn WifiModule(on_detail: EventHandler<Detail>) -> Element {
    let wifi = use_wifi()();
    let network = use_network()();
    let control = use_services().wifi_control;
    let state = radio_state(wifi.radio);
    rsx! {
        ModuleTile {
            glyph: wifi_glyph(&wifi, &network),
            title: "Wi-Fi",
            status: Some(Text::from(wifi_status(&wifi, &network))),
            state,
            chevron: Chevron::Detail,
            availability: radio_availability(&wifi),
            onclick: move |_| {
                // A stopped service drops it: the tile shows the radio it last published.
                let _gone = control.send(WifiCommand::Radio(radio_of(flipped(state))));
            },
            on_detail: move |_| on_detail.call(Detail::Wifi),
        }
    }
}

/// The tile's switch for the radio (`Blocked` shows off, and the switch is disabled).
pub(crate) fn radio_state(radio: Radio) -> ModuleState {
    match radio {
        Radio::On => ModuleState::On,
        Radio::Off | Radio::Blocked => ModuleState::Off,
    }
}

/// The radio a switch asks for.
pub(crate) fn radio_of(state: ModuleState) -> Radio {
    match state {
        ModuleState::On | ModuleState::Busy => Radio::On,
        ModuleState::Off => Radio::Off,
    }
}

/// Disabled without a wireless device or with the radio blocked in hardware.
pub(crate) fn radio_availability(wifi: &Wifi) -> Availability {
    match (wifi.device, wifi.radio) {
        (Presence::Absent, _) | (_, Radio::Blocked) => Availability::Disabled,
        (Presence::Present, Radio::On | Radio::Off) => Availability::Enabled,
    }
}

/// The signal's glyph while joined over Wi-Fi, otherwise plain Wi-Fi or Wi-Fi off.
pub(crate) fn wifi_glyph(wifi: &Wifi, network: &Network) -> Icon {
    match (wifi.radio, &network.link) {
        (Radio::On, Link::Wifi { .. }) => network_icon(network),
        (Radio::On, _) => Icon::Wifi,
        (Radio::Off | Radio::Blocked, _) => Icon::WifiOff,
    }
}

#[cfg(test)]
mod tests {
    use super::{radio_availability, radio_of, radio_state, wifi_glyph};
    use ds::{Availability, Icon, ModuleState};
    use sill_services::{Connectivity, Link, Network, Percent, Presence, Radio, Wifi};

    #[test]
    fn the_tile_follows_the_radio() {
        let wifi = |device, radio| Wifi {
            device,
            radio,
            ..Wifi::default()
        };
        let home = Network {
            link: Link::Wifi {
                ssid: "home".to_owned(),
                strength: Percent(90),
            },
            connectivity: Connectivity::Full,
        };
        // (device, radio, state, availability, glyph on `home`)
        let cases = [
            (
                Presence::Present,
                Radio::On,
                ModuleState::On,
                Availability::Enabled,
                Icon::WifiHigh,
            ),
            (
                Presence::Present,
                Radio::Off,
                ModuleState::Off,
                Availability::Enabled,
                Icon::WifiOff,
            ),
            (
                Presence::Present,
                Radio::Blocked,
                ModuleState::Off,
                Availability::Disabled,
                Icon::WifiOff,
            ),
            (
                Presence::Absent,
                Radio::Off,
                ModuleState::Off,
                Availability::Disabled,
                Icon::WifiOff,
            ),
        ];
        for (device, radio, state, availability, glyph) in cases {
            let w = wifi(device, radio);
            assert_eq!(radio_state(radio), state, "{radio:?}");
            assert_eq!(radio_availability(&w), availability, "{device:?} {radio:?}");
            assert_eq!(wifi_glyph(&w, &home), glyph, "{radio:?}");
        }
        assert_eq!(radio_of(ModuleState::On), Radio::On);
        assert_eq!(
            wifi_glyph(&wifi(Presence::Present, Radio::On), &Network::default()),
            Icon::Wifi
        );
    }
}
