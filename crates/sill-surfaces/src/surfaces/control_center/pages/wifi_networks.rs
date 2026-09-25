//! The Wi-Fi page: the radio `Toggle` in its header, then `listed_networks` as rows (signal
//! glyph by strength, name, [`crate::surfaces::control_center::network_detail`], a lock for a
//! secured one); choosing a saved network sends `WifiCommand::Connect`, a new one opens COSMIC
//! Settings' Wi-Fi page (no password prompt yet), the connected one `Disconnect`. Sends
//! `WifiCommand::Scan` when it opens. "Wi-Fi Settings…" at the foot.

use dioxus::prelude::*;
use ds::{Availability, Check, Icon};
use sill_services::{
    AccessPoint, Association, Presence, Radio, Saved, Security, Ssid, Wifi, WifiCommand,
    listed_networks,
};

use super::foot::{ClosePanel, EmptyLine, SettingsFoot, open_settings};
use super::head::DetailHead;
use crate::hooks::{use_services, use_wifi};
use crate::surfaces::control_center::modules::wifi::{radio_availability, radio_of, radio_state};
use crate::surfaces::control_center::network_detail;
use crate::surfaces::control_center::parts::{ModuleState, RowMark, SettingsRow, SettingsRows};

/// The Wi-Fi page.
#[component]
pub fn WifiNetworks(on_back: EventHandler<()>) -> Element {
    rsx! {
        WifiPane { back: Some(on_back) }
    }
}

/// What choosing a network row does.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum WifiPick {
    /// A saved network: join it.
    Join(Ssid),
    /// The connected network: leave it.
    Leave,
    /// A new network: COSMIC Settings asks for its password.
    Setup,
}

/// The Wi-Fi pane, with a back button in the control center and without one as the Wi-Fi bar
/// item's own dropdown.
#[component]
pub(crate) fn WifiPane(back: Option<EventHandler<()>>) -> Element {
    let wifi = use_wifi()();
    let services = use_services();
    let control = services.wifi_control.clone();
    let launch = services.launch.clone();
    let close = try_use_context::<ClosePanel>();
    use_hook({
        let control = control.clone();
        move || {
            // A stopped service drops it: the list shows what it last published.
            let _gone = control.send(WifiCommand::Scan);
        }
    });
    let rows = network_rows(&wifi);
    let empty = empty_line(&wifi, rows.len());
    let toggle = control.clone();
    rsx! {
        div { class: "sill-cc-detail",
            DetailHead {
                title: "Wi-Fi".to_owned(),
                back,
                state: radio_state(wifi.radio),
                availability: radio_availability(&wifi),
                on_toggle: move |to: ModuleState| {
                    let _gone = toggle.send(WifiCommand::Radio(radio_of(to)));
                },
            }
            if let Some(text) = empty {
                EmptyLine { text }
            } else {
                SettingsRows::<WifiPick> {
                    label: "Networks".to_owned(),
                    rows,
                    onpick: move |pick: WifiPick| {
                        let _gone = match pick {
                            WifiPick::Join(ssid) => control.send(WifiCommand::Connect(ssid)),
                            WifiPick::Leave => control.send(WifiCommand::Disconnect),
                            WifiPick::Setup => {
                                if let Some(ClosePanel(close)) = close {
                                    close.call(());
                                }
                                launch.send(open_settings())
                            }
                        };
                    },
                }
            }
            SettingsFoot { label: "Wi-Fi Settings…".to_owned() }
        }
    }
}

/// The list's rows: none unless the radio is on (`listed_networks` orders them: connected,
/// saved, then by strength).
pub(crate) fn network_rows(wifi: &Wifi) -> Vec<SettingsRow<WifiPick>> {
    match (wifi.device, wifi.radio) {
        (Presence::Present, Radio::On) => listed_networks(wifi)
            .into_iter()
            .map(|(ap, saved)| network_row(&ap, saved))
            .collect(),
        (Presence::Absent, _) | (_, Radio::Off | Radio::Blocked) => Vec::new(),
    }
}

/// One network's row.
pub(crate) fn network_row(ap: &AccessPoint, saved: Saved) -> SettingsRow<WifiPick> {
    let value = match (ap.association, saved) {
        (Association::Connected, _) => WifiPick::Leave,
        (Association::Connecting | Association::Idle, Saved::Known) => {
            WifiPick::Join(ap.ssid.clone())
        }
        (Association::Connecting | Association::Idle, Saved::New) => WifiPick::Setup,
    };
    let check = match ap.association {
        Association::Connected => Check::Checked,
        Association::Connecting | Association::Idle => Check::Unchecked,
    };
    let mark = match ap.security {
        Security::Secured => RowMark::Lock,
        Security::Open => RowMark::Nothing,
    };
    SettingsRow {
        value,
        glyph: signal_glyph(ap.strength.0),
        title: ap.ssid.0.clone(),
        detail: network_detail(ap, saved),
        check,
        mark,
        availability: Availability::Enabled,
    }
}

/// The signal's glyph (the bar's thresholds).
fn signal_glyph(strength: u16) -> Icon {
    match strength {
        0..=33 => Icon::WifiLow,
        34..=66 => Icon::Wifi,
        _ => Icon::WifiHigh,
    }
}

/// What the pane says instead of a list, if anything.
pub(crate) fn empty_line(wifi: &Wifi, rows: usize) -> Option<String> {
    let text = match (wifi.device, wifi.radio, rows) {
        (Presence::Absent, _, _) => "No Wi-Fi device",
        (_, Radio::Blocked, _) => "Wi-Fi is turned off by a switch",
        (_, Radio::Off, _) => "Wi-Fi is off",
        (_, Radio::On, 0) => "Looking for networks…",
        (_, Radio::On, _) => return None,
    };
    Some(text.to_owned())
}

#[cfg(test)]
mod tests {
    use super::{WifiPick, empty_line, network_row};
    use crate::surfaces::control_center::parts::RowMark;
    use ds::{Check, Icon};
    use sill_services::{
        AccessPoint, Association, Percent, Presence, Radio, Saved, Security, Ssid, Wifi,
    };

    fn ap(ssid: &str, strength: u16, association: Association, security: Security) -> AccessPoint {
        AccessPoint {
            ssid: Ssid(ssid.to_owned()),
            strength: Percent(strength),
            security,
            association,
        }
    }

    #[test]
    fn each_network_row_picks_by_what_it_is() {
        // (access point, saved, pick, check, mark, glyph, detail)
        let cases = [
            (
                ap("home", 80, Association::Connected, Security::Secured),
                Saved::Known,
                WifiPick::Leave,
                Check::Checked,
                RowMark::Lock,
                Icon::WifiHigh,
                "Connected",
            ),
            (
                ap("office", 50, Association::Idle, Security::Secured),
                Saved::Known,
                WifiPick::Join(Ssid("office".to_owned())),
                Check::Unchecked,
                RowMark::Lock,
                Icon::Wifi,
                "Saved",
            ),
            (
                ap("cafe", 20, Association::Idle, Security::Open),
                Saved::New,
                WifiPick::Setup,
                Check::Unchecked,
                RowMark::Nothing,
                Icon::WifiLow,
                "",
            ),
        ];
        for (ap, saved, pick, check, mark, glyph, detail) in cases {
            let row = network_row(&ap, saved);
            assert_eq!(
                (
                    row.value,
                    row.check,
                    row.mark,
                    row.glyph,
                    row.detail.as_str()
                ),
                (pick, check, mark, glyph, detail),
                "{}",
                ap.ssid.0
            );
        }
    }

    #[test]
    fn the_pane_says_why_there_is_no_list() {
        let wifi = |device, radio| Wifi {
            device,
            radio,
            ..Wifi::default()
        };
        let cases = [
            (Presence::Absent, Radio::Off, 0, Some("No Wi-Fi device")),
            (Presence::Present, Radio::Off, 0, Some("Wi-Fi is off")),
            (
                Presence::Present,
                Radio::Blocked,
                0,
                Some("Wi-Fi is turned off by a switch"),
            ),
            (
                Presence::Present,
                Radio::On,
                0,
                Some("Looking for networks…"),
            ),
            (Presence::Present, Radio::On, 3, None),
        ];
        for (device, radio, rows, want) in cases {
            assert_eq!(
                empty_line(&wifi(device, radio), rows).as_deref(),
                want,
                "{device:?} {radio:?} {rows}"
            );
        }
    }
}
