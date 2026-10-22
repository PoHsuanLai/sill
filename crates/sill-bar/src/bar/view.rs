//! The bar's document: `Ds` root with the workspace's SpaceLook, `Material::Bar`, the left group
//! (the system menu, the app's name and menus, the workspaces) and the right one, and the one
//! reusable menu popup's session in context.

use dioxus::prelude::*;
use ds::prelude::*;
use ds::style::appearance::blur::BlurState;
use shell_host::OutputInfo;

use super::app_name::AppName;
use super::app_titles::AppMenuTitles;
use super::clock::ClockItem;
use super::control_center_item::ControlCenterItem;
use super::hide::use_hide;
use super::io::use_bar;
use super::layer::layer;
use super::model::BarInput;
use super::module_item::ModuleItem;
use super::reveal::{BODY_ID, RevealEvent, STRIP_ID, lifted, timing};
use super::status_item::status_metrics;
use super::system_item::SystemItem;
use super::track::{Event, Target};
use super::tray::Tray;
use super::workspaces::WorkspaceIndicator;
use crate::control_center::menu_bar_modules;
use sill_services::TrayService;
use sill_ui_kit::autohide::Phase;
use sill_ui_kit::hooks::{use_appearance, use_follow_layer, use_service, use_settings};
use sill_ui_kit::look::use_active_look;
use sill_ui_kit::menu::PopupLook;
use sill_ui_kit::root::use_root_style;
use sill_ui_kit::shell_style::shell_metrics;

/// The bar's own stylesheet (lint-clean under `Profile::Strict`, `crates/sill/tests/lint.rs`).
pub const BAR_CSS: &str = include_str!("style.css");

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
    let root = use_root_style();
    let session = use_bar(&output, popup_look);
    let times = timing(&file, environment.settings.appearance.motion_level);
    let hide = use_hide(file.bar.autohide, times, session, output.id);
    use_follow_layer(layer(&output, &file));
    let state = *hide.machine.read();
    let lift = lifted(&state, (hide.drawn)(), &times, &file).0;
    let lifted_style = match lift {
        0 => String::new(),
        up => format!("transform:translateY(-{:.2}px)", f64::from(up) / 100.0),
    };
    let strip_up = matches!(state.phase, Phase::Hidden | Phase::Armed { .. });
    let strip_px = file.dock.autohide_trigger_strip_px.0;
    let modules = menu_bar_modules(&file.control_center);
    let outside = move |input: Event| session.feed(BarInput::Bar { input, title: None });
    let strip = format!("height:{}px", file.bar.height_px.0);
    let status = format!(
        "gap:{}px;{}",
        file.bar.status_gap_px.0,
        status_metrics(&file.bar).style_attr()
    );
    rsx! {
        div {
            class: "sill-bar-root",
            style: metrics.style_attr(),
            onmouseenter: move |_| hide.feed(RevealEvent::Enter),
            onmouseleave: move |_| hide.feed(RevealEvent::Leave),
        div { id: BODY_ID, class: "sill-bar-body", style: lifted_style,
        Ds {
            sheet: Some(sill_shell_kit::stylesheet()),
            appearance: popup_look.appearance,
            system: popup_look.system,
            look,
            material: Material::Bar,
            blur: BlurState::Available,
            tint_alpha: Some(popup_look.tint),
            stack: Some(popup_look.stack),
            surface: root.surface,
            user_style: root.user_style,
            AppStyle { css: BAR_CSS }
            div {
                class: "sill-bar",
                style: strip,
                onpointerdown: move |_| outside(Event::OutsidePress),
                onpointerup: move |_| outside(Event::Release(Target::Outside)),
                div { class: "sill-bar-group",
                    SystemItem { output: output.clone() }
                    AppName { output: output.clone() }
                    AppMenuTitles { output: output.clone() }
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
                    ClockItem { output }
                }
            }
        }
        }
        if strip_up {
            div { id: STRIP_ID, class: "sill-bar-strip", style: "height:{strip_px}px" }
        }
        }
    }
}
