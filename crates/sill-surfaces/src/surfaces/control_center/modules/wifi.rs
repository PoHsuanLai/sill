//! The Wi-Fi tile: a quire `ModuleTile` whose press turns the radio on or off,
//! [`crate::surfaces::control_center::wifi_status`] as its line, and its chevron to the network
//! list (`Detail::Wifi`). Reads `use_service::<WifiService>` and
//! `use_service::<NetworkService>`; sends `WifiCommand::Radio`.

use dioxus::prelude::*;
use ds::{Availability, Chevron, ModuleState, ModuleTile, StatusState, TextLine, WifiState};
use sill_model::network::Network;
use sill_model::units::Presence;
use sill_services::{Radio, Wifi, WifiCommand, WifiService};

use crate::hooks::{use_control, use_service};
use crate::surfaces::bar::glyph_state::{NetworkGlyph, network_glyph};
use crate::surfaces::control_center::modules::flipped;
use crate::surfaces::control_center::page::Detail;
use crate::surfaces::control_center::wifi_status;
use sill_services::NetworkService;

/// The Wi-Fi tile.
#[component]
pub fn WifiModule(on_detail: EventHandler<Detail>) -> Element {
    let wifi = use_service::<WifiService>()();
    let network = use_service::<NetworkService>()();
    let control = use_control::<WifiService>();
    let state = radio_state(wifi.radio);
    rsx! {
        ModuleTile {
            glyph: StatusState::Wifi(wifi_tile_state(&wifi, &network)),
            title: "Wi-Fi",
            status: Some(TextLine::from(wifi_status(&wifi, &network))),
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

/// The disc's Wi-Fi state: the bar item's (quire's `WifiGlyph`: searching while it joins, its
/// bars and badge once joined), off with the radio off, and not connected over a wire or
/// another link, which is not Wi-Fi.
pub(crate) fn wifi_tile_state(wifi: &Wifi, network: &Network) -> WifiState {
    match (wifi.radio, network_glyph(network, wifi.radio)) {
        (Radio::Off | Radio::Blocked, _) => WifiState::Off,
        (Radio::On, NetworkGlyph::Wifi(state)) => state,
        (Radio::On, NetworkGlyph::Plain(_)) => WifiState::Idle,
    }
}

#[cfg(test)]
mod tests {
    use super::{radio_availability, radio_of, radio_state, wifi_tile_state};
    use ds::detail::EventStamp;
    use ds::{Availability, ModuleState, WifiBars, WifiReach, WifiState};
    use sill_model::network::{Connectivity, Link, Network};
    use sill_model::{
        attempt::{Attempt, Attempts, OpStamp},
        units::{Percent, Presence},
    };
    use sill_services::{Radio, Wifi};

    fn wifi(device: Presence, radio: Radio) -> Wifi {
        Wifi {
            device,
            radio,
            ..Wifi::default()
        }
    }

    fn network(link: Link, now: Attempt) -> Network {
        Network {
            link,
            connectivity: Connectivity::Full,
            join: Attempts {
                now,
                minted: OpStamp(3),
            },
        }
    }

    #[test]
    fn the_tile_follows_the_radio() {
        // (device, radio, state, availability)
        let cases = [
            (
                Presence::Present,
                Radio::On,
                ModuleState::On,
                Availability::Enabled,
            ),
            (
                Presence::Present,
                Radio::Off,
                ModuleState::Off,
                Availability::Enabled,
            ),
            (
                Presence::Present,
                Radio::Blocked,
                ModuleState::Off,
                Availability::Disabled,
            ),
            (
                Presence::Absent,
                Radio::Off,
                ModuleState::Off,
                Availability::Disabled,
            ),
        ];
        for (device, radio, state, availability) in cases {
            let w = wifi(device, radio);
            assert_eq!(radio_state(radio), state, "{radio:?}");
            assert_eq!(radio_availability(&w), availability, "{device:?} {radio:?}");
        }
        assert_eq!(radio_of(ModuleState::On), Radio::On);
    }

    #[test]
    fn the_disc_draws_the_bar_items_wifi_state() {
        let home = Link::Wifi {
            ssid: "home".to_owned(),
            strength: Percent(90),
        };
        let joined = WifiState::Joined {
            bars: WifiBars::of(ds::Fraction(900)),
            reach: WifiReach::Internet,
        };
        // (name, radio, link, join, the disc's state)
        let cases = [
            ("joined", Radio::On, home.clone(), Attempt::Quiet, joined),
            (
                "radio off",
                Radio::Off,
                home.clone(),
                Attempt::Quiet,
                WifiState::Off,
            ),
            (
                "blocked",
                Radio::Blocked,
                Link::Disconnected,
                Attempt::Quiet,
                WifiState::Off,
            ),
            (
                "joining, not off (G300)",
                Radio::On,
                Link::Connecting,
                Attempt::Running(OpStamp(3)),
                WifiState::Joining(EventStamp(3)),
            ),
            (
                "idle",
                Radio::On,
                Link::Disconnected,
                Attempt::Quiet,
                WifiState::Idle,
            ),
            (
                "a wire is not Wi-Fi",
                Radio::On,
                Link::Wired,
                Attempt::Quiet,
                WifiState::Idle,
            ),
            (
                "nor another link",
                Radio::On,
                Link::Other,
                Attempt::Quiet,
                WifiState::Idle,
            ),
        ];
        for (name, radio, link, now, want) in cases {
            let got = wifi_tile_state(&wifi(Presence::Present, radio), &network(link, now));
            assert_eq!(got, want, "{name}");
        }
    }
}
