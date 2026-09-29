//! Which modules also sit in the bar as their own items (macOS's "Show in Menu Bar",
//! `control_center.menu_bar_*`), and in what order, pure. Each such item opens a dropdown
//! holding the same module component the control center shows (`PopupBody::Module`).

use sill_settings::{ControlCenterModule, ControlCenterSettings, InMenuBar};

/// The modules that can be bar items, left to right as the bar draws them: the status items
/// the bar always had keep their order (Sound, then Wi-Fi, then Battery), the new ones join
/// around them, and the control center's own item and the clock follow at the right end.
pub const MENU_BAR_ORDER: [ControlCenterModule; 6] = [
    ControlCenterModule::NowPlaying,
    ControlCenterModule::Display,
    ControlCenterModule::Sound,
    ControlCenterModule::Bluetooth,
    ControlCenterModule::Wifi,
    ControlCenterModule::Battery,
];

/// `module`'s `control_center.menu_bar_*` key; `Hide` for a module that cannot be a bar item
/// (Focus, Appearance).
pub fn in_menu_bar(cc: &ControlCenterSettings, module: ControlCenterModule) -> InMenuBar {
    match module {
        ControlCenterModule::Wifi => cc.menu_bar_wifi,
        ControlCenterModule::Bluetooth => cc.menu_bar_bluetooth,
        ControlCenterModule::Sound => cc.menu_bar_sound,
        ControlCenterModule::Display => cc.menu_bar_display,
        ControlCenterModule::Battery => cc.menu_bar_battery,
        ControlCenterModule::NowPlaying => cc.menu_bar_now_playing,
        ControlCenterModule::Focus
        | ControlCenterModule::KeyboardBrightness
        | ControlCenterModule::Appearance => InMenuBar::Hide,
    }
}

/// The modules shown as bar items, in [`MENU_BAR_ORDER`].
pub fn menu_bar_modules(cc: &ControlCenterSettings) -> Vec<ControlCenterModule> {
    MENU_BAR_ORDER
        .into_iter()
        .filter(|&module| in_menu_bar(cc, module) == InMenuBar::Show)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{in_menu_bar, menu_bar_modules};
    use sill_settings::{ControlCenterModule, ControlCenterSettings, InMenuBar};

    #[test]
    fn wifi_sound_and_battery_show_by_default_in_bar_order() {
        use ControlCenterModule::{Appearance, Battery, Bluetooth, NowPlaying, Sound, Wifi};
        let cc = ControlCenterSettings::default();
        assert_eq!(menu_bar_modules(&cc), vec![Sound, Wifi, Battery]);
        assert_eq!(in_menu_bar(&cc, Appearance), InMenuBar::Hide);
        let more = ControlCenterSettings {
            menu_bar_bluetooth: InMenuBar::Show,
            menu_bar_now_playing: InMenuBar::Show,
            menu_bar_sound: InMenuBar::Hide,
            ..ControlCenterSettings::default()
        };
        assert_eq!(
            menu_bar_modules(&more),
            vec![NowPlaying, Bluetooth, Wifi, Battery]
        );
    }
}
