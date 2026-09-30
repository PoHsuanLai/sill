//! The Wi-Fi page: the radio `Toggle` in its header, then `listed_networks` as rows (signal
//! glyph by strength on a disc, the accent disc for the network in use, name,
//! [`crate::control_center::network_detail`], a lock for a secured one); choosing a
//! saved network sends `WifiCommand::Connect`, a new one opens COSMIC Settings' Wi-Fi page (no
//! password prompt yet), the connected one `Disconnect`. The join's stamp (`Network.join`) is
//! the joining row's phase: a spinner where the lock was, the disc sealing when it lands, a
//! shake when it fails ([`crate::control_center::item_phase`]). Sends
//! `WifiCommand::Scan` when it opens. "Wi-Fi Settings…" at the foot.

use dioxus::prelude::*;
use ds::prelude::*;
use ds_shell::prelude::*;
use sill_model::attempt::Attempts;
use sill_model::units::Presence;
use sill_model::wifi::{
    AccessPoint, Association, Radio, Saved, Security, Ssid, Wifi, WifiCommand, listed_networks,
};

use super::feature_switch::{FeatureSwitch, back_button};
use super::foot::{ClosePanel, SettingsFoot, open_settings};
use crate::control_center::item_phase::{
    Aim, FAILED_WORDS, Item, ItemPhase, RowDisc, aim_after, item_phase,
};
use crate::control_center::modules::wifi::{radio_availability, radio_of, radio_state};
use crate::control_center::network_detail;
use crate::control_center::rows::{RowList, RowMark, RowSpec};
use sill_services::{NetworkService, WifiService};
use sill_ui_kit::hooks::{use_service, use_services};

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
    let wifi = use_service::<WifiService>()();
    let join = use_service::<NetworkService>()().join;
    let mut aim = use_hook(|| CopyValue::new(None::<Ssid>));
    let last = aim_after(aim.peek().clone(), joining(&wifi));
    aim.set(last.clone());
    let services = use_services();
    let control = services.wifi.control.clone();
    let launch = services.launch.control.clone();
    let close = try_use_context::<ClosePanel>();
    use_hook({
        let control = control.clone();
        move || {
            // A stopped service drops it: the list shows what it last published.
            let _gone = control.send(WifiCommand::Scan);
        }
    });
    let rows = network_rows(&wifi, join, last.as_ref());
    let empty = empty_title(&wifi, rows.len());
    let toggle = control.clone();
    rsx! {
        div { class: "sill-cc-detail",
            PaneHeader {
                title: "Wi-Fi",
                back: back_button(back),
                trailing: rsx! {
                    FeatureSwitch {
                        title: "Wi-Fi".to_owned(),
                        state: radio_state(wifi.radio),
                        availability: radio_availability(&wifi),
                        on_toggle: move |to: Check| {
                            let _gone = toggle.send(WifiCommand::Radio(radio_of(to)));
                        },
                    }
                },
            }
            if let Some(title) = empty {
                EmptyState { title, icon: Icon::Wifi }
            } else {
                RowList::<WifiPick> {
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

/// The network joining now, if any.
pub(crate) fn joining(wifi: &Wifi) -> Option<Ssid> {
    listed_networks(wifi)
        .into_iter()
        .find(|(ap, _)| ap.association == Association::Connecting)
        .map(|(ap, _)| ap.ssid)
}

/// The list's rows: none unless the radio is on (`listed_networks` orders them: connected,
/// saved, then by strength). `join` is the network service's join attempts, `last` the network
/// the pane last saw joining.
pub(crate) fn network_rows(
    wifi: &Wifi,
    join: Attempts,
    last: Option<&Ssid>,
) -> Vec<RowSpec<WifiPick>> {
    match (wifi.device, wifi.radio) {
        (Presence::Present, Radio::On) => listed_networks(wifi)
            .into_iter()
            .map(|(ap, saved)| {
                let aim = if last == Some(&ap.ssid) {
                    Aim::Last
                } else {
                    Aim::Other
                };
                network_row(&ap, saved, item_phase(join, item_of(ap.association), aim))
            })
            .collect(),
        (Presence::Absent, _) | (_, Radio::Off | Radio::Blocked) => Vec::new(),
    }
}

/// An access point's association as a row's item.
fn item_of(association: Association) -> Item {
    match association {
        Association::Idle => Item::Idle,
        Association::Connecting => Item::Connecting,
        Association::Connected => Item::InUse,
    }
}

/// One network's row in `phase`: the accent disc on the network in use (no check: the disc says
/// it), a lock on a secured one (the joining spinner takes its place), the failure's words in
/// the detail line.
pub(crate) fn network_row(ap: &AccessPoint, saved: Saved, phase: ItemPhase) -> RowSpec<WifiPick> {
    let value = match (ap.association, saved) {
        (Association::Connected, _) => WifiPick::Leave,
        (Association::Connecting | Association::Idle, Saved::Known) => {
            WifiPick::Join(ap.ssid.clone())
        }
        (Association::Connecting | Association::Idle, Saved::New) => WifiPick::Setup,
    };
    let mark = match ap.security {
        Security::Secured => RowMark::Lock,
        Security::Open => RowMark::Nothing,
    };
    let disc = match ap.association {
        Association::Connected => RowDisc::On,
        Association::Connecting | Association::Idle => RowDisc::Off,
    };
    let detail = match phase {
        ItemPhase::Failed(_) => FAILED_WORDS.to_owned(),
        ItemPhase::Rest | ItemPhase::Pending(_) | ItemPhase::Succeeded(_) => {
            network_detail(ap, saved)
        }
    };
    RowSpec {
        key: ap.ssid.0.clone(),
        value,
        glyph: signal_glyph(ap.strength.0),
        title: ap.ssid.0.clone(),
        detail,
        mark,
        phase,
        disc,
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

/// What the pane says instead of a list, if anything: the empty state's title.
pub(crate) fn empty_title(wifi: &Wifi, rows: usize) -> Option<String> {
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
    use super::{WifiPick, empty_title, joining, network_row, network_rows};
    use crate::control_center::item_phase::{ItemPhase, RowDisc};
    use crate::control_center::rows::RowMark;
    use ds::detail::EventStamp;
    use ds::prelude::*;
    use sill_model::wifi::{AccessPoint, Association, Radio, Saved, Security, Ssid, Wifi};
    use sill_model::{
        attempt::{Attempt, Attempts, OpStamp},
        units::{Percent, Presence},
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
        let e = EventStamp;
        // (access point, saved, phase, pick, mark, disc, glyph, detail)
        let cases = [
            (
                ap("home", 80, Association::Connected, Security::Secured),
                Saved::Known,
                ItemPhase::Rest,
                WifiPick::Leave,
                RowMark::Lock,
                RowDisc::On,
                Icon::WifiHigh,
                "Connected",
            ),
            (
                ap("office", 50, Association::Idle, Security::Secured),
                Saved::Known,
                ItemPhase::Rest,
                WifiPick::Join(Ssid("office".to_owned())),
                RowMark::Lock,
                RowDisc::Off,
                Icon::Wifi,
                "Saved",
            ),
            (
                ap("cafe", 20, Association::Idle, Security::Open),
                Saved::New,
                ItemPhase::Rest,
                WifiPick::Setup,
                RowMark::Nothing,
                RowDisc::Off,
                Icon::WifiLow,
                "",
            ),
            (
                ap("office", 50, Association::Idle, Security::Secured),
                Saved::Known,
                ItemPhase::Failed(e(2)),
                WifiPick::Join(Ssid("office".to_owned())),
                RowMark::Lock,
                RowDisc::Off,
                Icon::Wifi,
                "Couldn’t connect",
            ),
        ];
        for (ap, saved, phase, pick, mark, disc, glyph, detail) in cases {
            let row = network_row(&ap, saved, phase);
            assert_eq!(
                (
                    row.value,
                    row.mark,
                    row.disc,
                    row.glyph,
                    row.detail.as_str(),
                    row.phase
                ),
                (pick, mark, disc, glyph, detail, phase),
                "{}",
                ap.ssid.0
            );
        }
    }

    #[test]
    fn the_joining_row_takes_the_joins_stamp_and_keeps_it_when_it_lands() {
        let mut wifi = sill_model::wifi::Wifi {
            device: Presence::Present,
            radio: Radio::On,
            access_points: vec![
                ap("home", 80, Association::Idle, Security::Secured),
                ap("office", 50, Association::Connecting, Security::Secured),
            ],
            saved: vec![Ssid("home".to_owned()), Ssid("office".to_owned())],
        };
        let office = Ssid("office".to_owned());
        assert_eq!(joining(&wifi), Some(office.clone()));
        let phases = |wifi: &Wifi, now: Attempt| {
            network_rows(
                wifi,
                Attempts {
                    now,
                    minted: OpStamp(4),
                },
                Some(&office),
            )
            .into_iter()
            .map(|row| (row.key, row.phase))
            .collect::<Vec<_>>()
        };
        let row = |name: &str, phase| (name.to_owned(), phase);
        assert_eq!(
            phases(&wifi, Attempt::Running(OpStamp(4))),
            vec![
                row("home", ItemPhase::Rest),
                row("office", ItemPhase::Pending(EventStamp(4)))
            ],
        );
        wifi.access_points[1].association = Association::Connected;
        assert_eq!(
            phases(&wifi, Attempt::Quiet),
            vec![
                row("office", ItemPhase::Succeeded(EventStamp(4))),
                row("home", ItemPhase::Rest)
            ],
        );
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
                empty_title(&wifi(device, radio), rows).as_deref(),
                want,
                "{device:?} {radio:?} {rows}"
            );
        }
    }
}
