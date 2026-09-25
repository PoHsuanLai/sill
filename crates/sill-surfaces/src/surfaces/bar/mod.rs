//! The bar (design/20 §1.1, design/13 §13.3.1-13.3.4, 06 §20.3): one per output, left the app
//! name and the workspace indicator, right the tray, the control center modules shown as their
//! own items (`control_center.menu_bar_*`: Sound, Wi-Fi and Battery by default), the control
//! center's item and the clock; every menu an `xdg_popup`, tracked by [`menu_track`].

pub mod app_name;
pub mod clock;
pub mod control_center_item;
pub mod layer;
pub mod menu_track;
pub mod module_item;
pub mod popup;
pub mod tray;
pub mod workspaces;

mod content;
mod geometry;
mod menu_popup;
mod menus;
mod pick;
mod plan;
mod reorder;
mod session;
pub(crate) mod shell_style;
pub(crate) mod status;
mod status_item;
mod title;
mod tray_input;
mod tray_menu;

pub use layer::bar_layer;

use dioxus::prelude::*;
use ds::{BlurState, Ds, Material, Point, Px, Rect, Size, SpaceLook};
use shell_host::{OutputInfo, SurfaceHandle};
use sill_services::{OutputName, WorkspaceKey, Workspaces};
use sill_settings::{BarSettings, FocusLink, FocusMode, SettingsFeed, SpacesSettings};

use app_name::AppName;
use clock::ClockItem;
use content::{BarState, front_app};
use control_center_item::ControlCenterItem;
use menu_track::{MenuIn, MenuParams, Target};
use menus::BarMenu;
use module_item::ModuleItem;
use session::{BarInput, MenuBar, PopupLook, SessionDeps, use_menu_session};
use shell_style::shell_metrics;
use status_item::status_metrics;
use tray::Tray;
use workspaces::{WorkspaceIndicator, group_for};

use crate::hooks::{
    use_appearance, use_apps, use_clock, use_services, use_settings, use_space_look, use_toplevels,
    use_tray, use_workspaces,
};
use crate::surfaces::control_center::{ControlCenterLink, height_cap, menu_bar_modules};

/// The bar's own stylesheet (lint-clean under `Profile::Strict`, `tests/bar_lint.rs`).
pub(crate) const BAR_CSS: &str = include_str!("../../style/bar.css");

/// The look of the active workspace on `output` (design/21 §5 rule 6: each output's bar follows
/// that output's active workspace); the first preset while there is none (KWin).
fn active_look(
    workspaces: &Workspaces,
    output: Option<&OutputName>,
    spaces: &SpacesSettings,
) -> SpaceLook {
    let active = group_for(&workspaces.groups, output).and_then(|group| {
        group
            .workspaces
            .iter()
            .position(|w| w.activity == sill_services::Activity::Active)
            .map(|index| (group.workspaces[index].key, index))
    });
    let (key, index) = active.unwrap_or((WorkspaceKey(0), 0));
    use_space_look(key, index, spaces)
}

/// The bar on `output`: `Ds` root with the workspace's SpaceLook, `Material::Bar`, the left
/// and right groups, and the one reusable menu popup. The root draws the Space gradient at the
/// Bar's tint alpha and puts its content on the frame ground (quire's `FrameTint::Tinted` and
/// `Ground::Frame`, the Bar's defaults), so the bar paints no tint of its own.
#[component]
pub fn Bar(output: OutputInfo) -> Element {
    let ds_scale = crate::hooks::use_ds_scale();
    let file = use_settings()();
    let environment = use_appearance()();
    let workspaces = use_workspaces()();
    let tray = use_tray()();
    let on = output.name.clone().map(OutputName);
    let look = active_look(&workspaces, on.as_ref(), &file.spaces);
    let metrics = shell_metrics(&file);
    let popup_look = PopupLook {
        appearance: environment.settings.appearance.appearance(),
        system: environment.system,
        tint: environment.tint_alpha(),
        metrics,
        stack: environment.material_stack(),
    };
    let state = BarState {
        app: front_app(&use_toplevels()(), &use_apps()(), on.as_ref()),
        clock: use_clock()(),
        control_center: file.control_center.clone(),
    };
    let focus = try_use_context::<FocusLink>().unwrap_or_else(|| FocusLink::fixed(FocusMode::Off));
    let bar = use_menu_session(SessionDeps {
        state,
        services: use_services(),
        surface: try_use_context::<SurfaceHandle>(),
        params: MenuParams::new(&file.menus),
        look: popup_look,
        namespace: bar_layer(&output, &file.bar).namespace,
        output: output.id,
        power_menu: try_use_context::<crate::surfaces::power_menu::PowerMenuLink>(),
        settings: use_context::<SettingsFeed>(),
        focus,
        output_name: on.clone(),
        bar_width: output.logical.width,
        cap: height_cap(output.logical.height, &file.bar, &file.control_center),
        status_box: status_box(output.logical.width, &file.bar),
    });
    use_context_provider(|| bar);
    use_control_center_asks(bar, output.id);
    let modules = menu_bar_modules(&file.control_center);
    let outside = move |input: MenuIn| bar.feed(BarInput::Bar { input, title: None });
    let strip = format!("height:{}px", file.bar.height_px.0);
    let status = format!(
        "gap:{}px;{}",
        file.bar.status_gap_px.0,
        status_metrics(&file.bar).style_attr()
    );
    rsx! {
        div { class: "sill-bar-root", style: metrics.style_attr(),
        Ds {
            scale: ds_scale,
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
                        ModuleItem { key: "{module:?}", output: output.clone(), module }
                    }
                    ControlCenterItem { output: output.clone() }
                    ClockItem { output: output.clone() }
                }
            }
        }
        }
    }
}

/// Follow `sill control-center toggle` (the daemon's `ControlCenterLink`, absent in a demo or a
/// test): an ask for this bar's output toggles its control center.
fn use_control_center_asks(bar: MenuBar, output: shell_host::OutputId) {
    let link = try_use_context::<ControlCenterLink>();
    use_hook(move || {
        let Some(link) = link else {
            return;
        };
        let mut asks = link.subscribe();
        // Only asks made after the bar subscribed.
        asks.borrow_and_update();
        spawn(async move {
            while asks.changed().await.is_ok() {
                let ask = *asks.borrow_and_update();
                if ask.output == Some(output) {
                    bar.toggle(BarMenu::ControlCenter);
                }
            }
        });
    });
}

/// The status items' box, vertically centred in the bar, at its right end: the control center's
/// title rect when it opens with no pointer event behind it.
fn status_box(bar_width: i32, bar: &BarSettings) -> Rect {
    let side = f32::from(bar.status_icon_box_px.0);
    let height = f32::from(bar.height_px.0);
    Rect {
        origin: Point {
            x: Px(bar_width as f32 - side),
            y: Px((height - side) / 2.0),
        },
        size: Size {
            width: Px(side),
            height: Px(side),
        },
    }
}
