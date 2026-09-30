//! The bar's document: `Ds` root with the workspace's SpaceLook, `Material::Bar`, the left and
//! right groups, and the one reusable menu popup's session in context.

use dioxus::prelude::*;
use ds::{BlurState, Ds, Material};
use shell_host::OutputInfo;

use super::app_name::AppName;
use super::clock::ClockItem;
use super::control_center_item::ControlCenterItem;
use super::io::use_bar;
use super::menu_track::{MenuIn, Target};
use super::model::BarInput;
use super::module_item::ModuleItem;
use super::shell_style::shell_metrics;
use super::status_item::status_metrics;
use super::tray::Tray;
use super::workspaces::WorkspaceIndicator;
use crate::hooks::{use_appearance, use_service, use_settings};
use crate::look::use_active_look;
use crate::menu::PopupLook;
use crate::surfaces::control_center::menu_bar_modules;
use sill_services::TrayService;

/// The bar's own stylesheet (lint-clean under `Profile::Strict`, `crates/sill/tests/lint.rs`).
pub const BAR_CSS: &str = include_str!("../../style/bar.css");

/// The bar on `output`. The root draws the Space gradient at the Bar's tint alpha and puts its
/// content on the frame ground (quire's `FrameTint::Tinted` and `Ground::Frame`, the Bar's
/// defaults), so the bar paints no tint of its own.
pub(super) fn bar(output: OutputInfo) -> Element {
    let file = use_settings()();
    let environment = use_appearance()();
    let tray = use_service::<TrayService>()();
    let look = use_active_look(&output);
    let metrics = shell_metrics(&file);
    let popup_look = PopupLook {
        appearance: environment.settings.appearance.appearance(),
        system: environment.system,
        tint: environment.tint_alpha(),
        metrics,
        stack: environment.material_stack(),
    };
    let session = use_bar(&output, popup_look);
    let modules = menu_bar_modules(&file.control_center);
    let outside = move |input: MenuIn| session.feed(BarInput::Bar { input, title: None });
    let strip = format!("height:{}px", file.bar.height_px.0);
    let status = format!(
        "gap:{}px;{}",
        file.bar.status_gap_px.0,
        status_metrics(&file.bar).style_attr()
    );
    rsx! {
        div { class: "sill-bar-root", style: metrics.style_attr(),
        Ds {
            sheet: Some(ds_shell::stylesheet()),
            appearance: popup_look.appearance,
            system: popup_look.system,
            look,
            material: Material::Bar,
            blur: BlurState::Available,
            tint_alpha: Some(popup_look.tint),
            stack: Some(popup_look.stack),
            style { {BAR_CSS} }
            div {
                class: "sill-bar",
                style: strip,
                onpointerdown: move |_| outside(MenuIn::OutsidePress),
                onpointerup: move |_| outside(MenuIn::Release(Target::Outside)),
                div { class: "sill-bar-group",
                    AppName { output: output.clone() }
                    WorkspaceIndicator { output: output.clone() }
                }
                div { class: "sill-bar-group", style: status,
                    if !tray.items.is_empty() {
                        Tray { output: output.clone() }
                    }
                    for module in modules {
                        ModuleItem { key: "{module:?}", module }
                    }
                    ControlCenterItem { output: output.clone() }
                    ClockItem { output: output.clone() }
                }
            }
        }
        }
    }
}
