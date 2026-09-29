//! How big the bar's popup is for the control center or a module's own dropdown, pure: the
//! bar's geometry asks this for the box inside the popup's shadow room.

use shell_host::Size;
use sill_model::units::Presence;
use sill_services::{Bluetooth, BtPower, Radio, Wifi};
use sill_settings::{ControlCenterModule, ControlCenterSettings};

use super::geometry::{estimate_size, place};
use super::page::{CcPage, Detail};
use super::sizes::{detail_height, module_height};

/// What the estimate needs beyond the settings: the page on show, how many rows each detail
/// list has, whether the machine has a keyboard backlight (its module is drawn only then), and
/// the panel's cap ([`super::height_cap`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Fit {
    pub page: CcPage,
    pub wifi_rows: usize,
    pub device_rows: usize,
    pub keyboard: Presence,
    pub cap: i32,
}

/// The control center's box.
pub(crate) fn panel_size(cc: &ControlCenterSettings, fit: &Fit) -> Size {
    let rows = match fit.page {
        CcPage::Modules => 0,
        CcPage::Detail(Detail::Wifi) => fit.wifi_rows,
        CcPage::Detail(Detail::Bluetooth) => fit.device_rows,
    };
    let placed = place(&drawn_modules(&cc.modules, fit.keyboard), cc.grid_columns.0);
    estimate_size(cc, fit.page, &placed, rows, fit.cap)
}

/// `module`'s own dropdown's box: Wi-Fi's and Bluetooth's detail panes, or the module.
pub(crate) fn dropdown_size(
    cc: &ControlCenterSettings,
    module: ControlCenterModule,
    fit: &Fit,
) -> Size {
    let inner = match module {
        ControlCenterModule::Wifi => detail_height(fit.wifi_rows),
        ControlCenterModule::Bluetooth => detail_height(fit.device_rows),
        ControlCenterModule::Focus
        | ControlCenterModule::Display
        | ControlCenterModule::KeyboardBrightness
        | ControlCenterModule::Sound
        | ControlCenterModule::NowPlaying
        | ControlCenterModule::Appearance
        | ControlCenterModule::Battery => module_height(module),
    };
    Size {
        width: i32::from(cc.width_px.0),
        height: (inner + 2 * i32::from(cc.grid_padding_px.0)).min(fit.cap),
    }
}

/// The modules the grid draws: `modules` less the Keyboard Brightness module on a machine with
/// no keyboard backlight (it draws nothing there).
pub(crate) fn drawn_modules(
    modules: &[ControlCenterModule],
    keyboard: Presence,
) -> Vec<ControlCenterModule> {
    modules
        .iter()
        .copied()
        .filter(|module| match (module, keyboard) {
            (ControlCenterModule::KeyboardBrightness, Presence::Absent) => false,
            (ControlCenterModule::KeyboardBrightness, Presence::Present)
            | (
                ControlCenterModule::Wifi
                | ControlCenterModule::Bluetooth
                | ControlCenterModule::Focus
                | ControlCenterModule::Display
                | ControlCenterModule::Sound
                | ControlCenterModule::NowPlaying
                | ControlCenterModule::Appearance
                | ControlCenterModule::Battery,
                _,
            ) => true,
        })
        .collect()
}

/// About how many rows the Wi-Fi list has: its distinct named networks while the radio is on
/// (`listed_networks` keeps one row per name).
pub(crate) fn wifi_rows(wifi: &Wifi) -> usize {
    match (wifi.device, wifi.radio) {
        (Presence::Present, Radio::On) => {
            let mut names: Vec<&str> = wifi
                .access_points
                .iter()
                .map(|ap| ap.ssid.0.as_str())
                .filter(|name| !name.is_empty())
                .collect();
            names.sort_unstable();
            names.dedup();
            names.len()
        }
        (Presence::Absent, _) | (_, Radio::Off | Radio::Blocked) => 0,
    }
}

/// About how many rows the Bluetooth list has: every device while the adapter is on.
pub(crate) fn device_rows(bluetooth: &Bluetooth) -> usize {
    match (bluetooth.presence, bluetooth.power) {
        (Presence::Present, BtPower::On) => bluetooth.devices.len(),
        (Presence::Absent, _) | (_, BtPower::Off) => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::{Fit, device_rows, drawn_modules, dropdown_size, panel_size, wifi_rows};
    use crate::surfaces::control_center::{CcPage, Detail};
    use sill_model::units::{Percent, Presence};
    use sill_services::{AccessPoint, Association, Bluetooth, Radio, Security, Ssid, Wifi};
    use sill_settings::{ControlCenterModule, ControlCenterSettings};

    #[test]
    fn the_popup_fits_the_page_or_the_module() {
        let cc = ControlCenterSettings::default();
        let fit = |page| Fit {
            page,
            wifi_rows: 4,
            device_rows: 0,
            keyboard: Presence::Absent,
            cap: 1032,
        };
        let grid = panel_size(&cc, &fit(CcPage::Modules));
        let wifi = panel_size(&cc, &fit(CcPage::Detail(Detail::Wifi)));
        assert_eq!(grid.width, 320);
        assert!(grid.height > wifi.height, "{grid:?} {wifi:?}");
        assert_eq!(wifi.height, 20 + 36 + 6 + 4 * 44 + 6 + 29);
        let sound = dropdown_size(&cc, ControlCenterModule::Sound, &fit(CcPage::Modules));
        assert_eq!((sound.width, sound.height), (320, 20 + 64));
        let list = dropdown_size(&cc, ControlCenterModule::Wifi, &fit(CcPage::Modules));
        assert_eq!(list.height, wifi.height);
    }

    #[test]
    fn rows_are_counted_only_while_the_radio_is_on() {
        let ap = |name: &str| AccessPoint {
            ssid: Ssid(name.to_owned()),
            strength: Percent(50),
            security: Security::Secured,
            association: Association::Idle,
        };
        let wifi = Wifi {
            device: Presence::Present,
            radio: Radio::On,
            access_points: vec![ap("home"), ap("home"), ap(""), ap("cafe")],
            saved: Vec::new(),
        };
        assert_eq!(wifi_rows(&wifi), 2);
        let off = Wifi {
            radio: Radio::Off,
            ..wifi
        };
        assert_eq!(wifi_rows(&off), 0);
        assert_eq!(device_rows(&Bluetooth::default()), 0);
    }

    #[test]
    fn the_keyboard_module_is_drawn_only_with_a_backlight() {
        use ControlCenterModule::{Display, KeyboardBrightness, Sound};
        let modules = [Display, KeyboardBrightness, Sound];
        let cases = [
            (Presence::Present, vec![Display, KeyboardBrightness, Sound]),
            (Presence::Absent, vec![Display, Sound]),
        ];
        for (keyboard, want) in cases {
            assert_eq!(drawn_modules(&modules, keyboard), want, "{keyboard:?}");
        }
        let cc = ControlCenterSettings::default();
        let fit = |keyboard| Fit {
            page: CcPage::Modules,
            wifi_rows: 0,
            device_rows: 0,
            keyboard,
            cap: 2000,
        };
        let with = panel_size(&cc, &fit(Presence::Present)).height;
        let without = panel_size(&cc, &fit(Presence::Absent)).height;
        assert_eq!(with - without, 64 + 10, "a level module and its gap");
    }
}
