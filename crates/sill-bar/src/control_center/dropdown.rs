//! A module's own bar dropdown (`control_center.menu_bar_*`, design/13 §13.3.7 "Module bar
//! items"): a click on the module's bar item opens that module's detail directly. Wi-Fi and
//! Bluetooth open their detail panes (no back button); the others show their module on a `Bare`
//! `ModulePanel`, the popover card being its plate.

use dioxus::prelude::*;
use sill_model::ids::OutputName;
use sill_settings::ControlCenterModule;
use sill_shell_kit::control_center::module_panel::PanelPlate;

use super::modules::CcCx;
use super::modules::bluetooth_ask::{TurnOffAlert, provide_power_ask};
use super::pages::foot::ClosePanel;
use super::panel::{PanelCap, panel_style, use_keyboard_home};
use sill_ui_kit::hooks::use_settings;

/// `module`'s dropdown on the bar of `output`; `on_close` closes the popup (a pick that opens
/// Settings).
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
    provide_power_ask();
    let cx = CcCx {
        output,
        on_detail: EventHandler::new(|_| {}),
        on_back: None,
    };
    let body = super::modules::module(module).dropdown(&cx);
    rsx! {
        div {
            class: "sill-cc",
            "data-drop": "module",
            tabindex: "-1",
            style: panel_style(cc.width_px.0, cc.grid_padding_px.0, cap),
            onmounted: move |event: MountedEvent| home.mounted(event),
            onfocusin: move |_| home.arrived(),
            onfocusout: move |_| home.left(),
            {body}
            TurnOffAlert { on_answered: move |()| home.take() }
        }
    }
}
