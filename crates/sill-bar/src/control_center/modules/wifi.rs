//! The Wi-Fi tile: a quire `ModuleTile` whose press turns the radio on or off,
//! [`crate::control_center::wifi_status`] as its line, and its chevron to the network
//! list (`Detail::Wifi`). Reads `use_service::<WifiService>` and
//! `use_service::<NetworkService>`; sends `WifiCommand::Radio`.

use dioxus::prelude::*;
use ds::Check;
use ds::{Availability, StatusState, TextLine, WifiState};
use ds_shell::ModuleTile;
use sill_model::network::Network;
use sill_model::units::Presence;
use sill_model::wifi::{Radio, Wifi, WifiCommand};
use sill_services::WifiService;

use super::cc_module::{BarItemFace, CcCx, CcModule, Span};
use crate::bar::{BarMenu, StatusIcon};
use crate::control_center::modules::flipped;
use crate::control_center::page::Detail;
use crate::control_center::pages::wifi_networks::WifiPane;
use crate::control_center::wifi_status;
use sill_services::NetworkService;
use sill_settings::{ControlCenterSettings, InMenuBar};
use sill_ui_kit::hooks::{use_control, use_service};
use sill_ui_kit::status::glyph::{NetworkGlyph, network_glyph};
use sill_ui_kit::status::lines::network_lines;

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
            value: state,
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
pub(crate) fn radio_state(radio: Radio) -> Check {
    match radio {
        Radio::On => Check::On,
        Radio::Off | Radio::Blocked => Check::Off,
    }
}

/// The radio a switch asks for.
pub(crate) fn radio_of(state: Check) -> Radio {
    match state {
        Check::On | Check::Mixed => Radio::On,
        Check::Off => Radio::Off,
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

/// The Wi-Fi item in the bar: the Wi-Fi glyph (searching while it joins, its bars and badge once
/// joined, the slash with the radio off), or the wire's.
#[component]
fn WifiItem() -> Element {
    let network = use_service::<NetworkService>()();
    let radio = use_service::<WifiService>()().radio;
    let [link, _] = network_lines(&network);
    rsx! {
        StatusIcon { menu: BarMenu::Network, icon: network_glyph(&network, radio), tooltip: link }
    }
}

/// The Wi-Fi module as the control center and the bar draw it.
pub(crate) struct WifiCc;

impl CcModule for WifiCc {
    fn span(&self) -> Span {
        Span::One
    }

    fn tile(&self, cx: &CcCx) -> Element {
        rsx! { WifiModule { on_detail: cx.on_detail } }
    }

    fn dropdown(&self, _cx: &CcCx) -> Option<Element> {
        Some(rsx! { WifiPane { back: None } })
    }

    fn menu_bar(&self, settings: &ControlCenterSettings) -> InMenuBar {
        settings.menu_bar_wifi
    }

    fn bar_item(&self) -> Option<BarItemFace> {
        Some(BarItemFace {
            menu: BarMenu::Network,
            item: || rsx! { WifiItem {} },
        })
    }

    fn detail(&self, page: Detail, cx: &CcCx) -> Option<Element> {
        (page == Detail::Wifi).then(|| rsx! { WifiPane { back: cx.on_back } })
    }
}

#[cfg(test)]
mod tests {
    use super::{radio_availability, radio_of, radio_state, wifi_tile_state};
    use ds::Check;
    use ds::detail::EventStamp;
    use ds::{Availability, WifiBars, WifiReach, WifiState};
    use sill_model::network::{Connectivity, Link, Network};
    use sill_model::wifi::{Radio, Wifi};
    use sill_model::{
        attempt::{Attempt, Attempts, OpStamp},
        units::{Percent, Presence},
    };

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
                Check::On,
                Availability::Enabled,
            ),
            (
                Presence::Present,
                Radio::Off,
                Check::Off,
                Availability::Enabled,
            ),
            (
                Presence::Present,
                Radio::Blocked,
                Check::Off,
                Availability::Disabled,
            ),
            (
                Presence::Absent,
                Radio::Off,
                Check::Off,
                Availability::Disabled,
            ),
        ];
        for (device, radio, state, availability) in cases {
            let w = wifi(device, radio);
            assert_eq!(radio_state(radio), state, "{radio:?}");
            assert_eq!(radio_availability(&w), availability, "{device:?} {radio:?}");
        }
        assert_eq!(radio_of(Check::On), Radio::On);
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
