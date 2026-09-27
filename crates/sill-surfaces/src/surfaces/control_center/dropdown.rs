//! A module's own bar dropdown (`control_center.menu_bar_*`, design/13 §13.3.7 "Module bar
//! items"): a click on the module's bar item opens that module's detail directly. Wi-Fi and
//! Bluetooth open their detail panes (no back button); the others show their module on a `Bare`
//! `ModulePanel`, the popover card being its plate.

use dioxus::prelude::*;
use ds::PanelPlate;
use sill_services::OutputName;
use sill_settings::ControlCenterModule;

use super::modules::Module;
use super::pages::bluetooth_devices::BluetoothPane;
use super::pages::foot::ClosePanel;
use super::pages::wifi_networks::WifiPane;
use super::panel::{PanelCap, panel_style, use_keyboard_home};
use crate::hooks::use_settings;

/// `module`'s dropdown on the bar of `output`; `on_close` closes the popup (a pick that opens
/// COSMIC Settings).
#[component]
pub fn ModuleDropdown(
    module: ControlCenterModule,
    output: Option<OutputName>,
    on_close: EventHandler<()>,
) -> Element {
    let cc = use_settings()().control_center;
    let cap = try_use_context::<PanelCap>();
    let home = use_keyboard_home();
    use_context_provider(|| ClosePanel(Callback::new(move |()| on_close.call(()))));
    use_context_provider(|| PanelPlate::Bare);
    let body = match module {
        ControlCenterModule::Wifi => rsx! { WifiPane { back: None } },
        ControlCenterModule::Bluetooth => rsx! { BluetoothPane { back: None } },
        ControlCenterModule::Focus
        | ControlCenterModule::Display
        | ControlCenterModule::KeyboardBrightness
        | ControlCenterModule::Sound
        | ControlCenterModule::NowPlaying
        | ControlCenterModule::Appearance
        | ControlCenterModule::Battery => rsx! {
            Module { module, output, on_detail: move |_| {} }
        },
    };
    rsx! {
        div {
            class: "sill-cc",
            "data-drop": "module",
            tabindex: "-1",
            style: panel_style(cc.width_px.0, cc.grid_padding_px.0, cap),
            onmounted: move |event: MountedEvent| home.mounted(event),
            {body}
        }
    }
}
