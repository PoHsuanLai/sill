//! The bar popup's own document: a `Ds` root holding one ds
//! `Menu{Dropdown}` (or, for the control center and a module's own item, a ds `Popover` around
//! `control_center::ControlCenter` or the module),
//! following the bar's view channel: a new view swaps the contents and repositions the popup
//! (`xdg_popup.reposition`), none closes it. The root is `Material::Popover`, which paints
//! nothing on its own box (quire's transparent popup root); the menu card paints the material.
//! The menu opens with no entrance (design/13 §13.3.2) and reports the pointer's hover and
//! release to the bar's tracking machine.

use dioxus::prelude::*;
use ds::{
    Align, Anchor, BlurState, Ds, Elevation, Material, Menu, MenuEntrance, MenuKind, Placement,
    Point, Popover, Press, Px, Side,
};
use shell_host::{PopupPlacement, SurfaceOp, use_surface, use_watch};
use sill_model::ids::OutputName;
use sill_settings::ControlCenterModule;
use tokio::sync::{mpsc, watch};

use super::content::PopupBody;
use super::geometry::{MARGIN, frame_style};
use super::model::PopupView;
use super::pick::BarPick;
use super::plan::{PopupReport, choice};
use crate::root::HostModalityBridge;
use crate::surfaces::control_center::{CcPage, ControlCenter, ModuleDropdown};

/// The popup's link to its bar, provided as its root context.
#[derive(Debug, Clone)]
pub(crate) struct PopupLink {
    pub view: watch::Receiver<Option<PopupView>>,
    pub reports: mpsc::UnboundedSender<PopupReport>,
    /// The control center's page changes, which resize the popup.
    pub pages: mpsc::UnboundedSender<CcPage>,
    /// The bar's output (the Display module's brightness target).
    pub output: Option<OutputName>,
}

/// The anchor that puts a ds floating surface `MARGIN` inside the frame's top-left corner, so at
/// (`SIDE`, `TOP`) in the popup: `place` clamps it there whatever its size, since the frame is
/// the surface plus the margin.
fn corner() -> Anchor {
    let at = f32::from(u8::try_from(MARGIN).unwrap_or(8));
    Anchor::Point(Point {
        x: Px(at),
        y: Px(1.0),
    })
}

/// The popup document's own stylesheet: the frame its `Ds` root sits in.
pub const POPUP_CSS: &str = include_str!("../../style/bar_popup.css");

/// The control center's and the module dropdowns' stylesheet (lint-clean under
/// `Profile::Strict`, `tests/control_center_lint.rs`).
pub(crate) const CONTROL_CENTER_CSS: &str = include_str!("../../style/control_center.css");

/// The gap from [`corner`]'s point to the menu's top edge.
const GAP: Px = Px(7.0);

/// The popup root. A popup is a `VirtualDom` of its own and inherits nothing from the bar, so
/// it provides quire's rect measurer itself.
#[component]
pub(crate) fn BarPopup() -> Element {
    let ds_scale = crate::hooks::use_ds_scale();
    ds_native::measure::provide();
    ds_native::focus::provide();
    let link = use_context::<PopupLink>();
    let surface = use_surface();
    let view = use_watch(link.view.clone());
    let mut placed = use_hook(|| CopyValue::new(None::<PopupPlacement>));
    let reports = link.reports.clone();
    use_drop(move || {
        // The bar may be gone first; then there is nobody to tell.
        let _ = reports.send(PopupReport::Gone);
    });
    use_effect(move || match view() {
        None => surface.close(),
        Some(next) => {
            let before = placed.replace(Some(next.placement));
            if before.is_some_and(|before| before != next.placement) {
                surface.apply(SurfaceOp::Reposition(next.placement));
            }
        }
    });
    let Some(current) = view() else {
        return rsx! {};
    };
    let look = current.look;
    let which = current.menu;
    let content = match current.body {
        PopupBody::Menu(entries) => rsx! {
            BarMenuList { key: "{which:?}", entries }
        },
        PopupBody::ControlCenter(_) => rsx! {
            PanelPopover { module: None }
        },
        PopupBody::Module(module) => rsx! {
            PanelPopover { module: Some(module) }
        },
    };
    rsx! {
        style { {POPUP_CSS} }
        style { {CONTROL_CENTER_CSS} }
        HostModalityBridge {
            div { class: "sill-popup-frame", style: "{frame_style()};{look.metrics.style_attr()}",
                Ds {
                    scale: ds_scale,
                    appearance: look.appearance,
                    system: look.system,
                    material: Material::Popover,
                    blur: BlurState::Unavailable,
                    tint_alpha: Some(look.tint),
                    stack: Some(look.stack),
                    {content}
                }
            }
        }
    }
}

/// A ds `Menu{Dropdown}`, at once (no entrance: a bar menu, and every hover switch); the root
/// keys it by the bar menu so a switch starts a fresh selection. Its hover and releases go to
/// the tracking machine, which decides a press-drag-release; quire's own pick then finds the
/// session closed. The keyboard's pick still arrives as `Picked`, and quire calls `onpick`
/// before `onclose`, so a close reports a dismissal only when nothing was picked.
#[component]
fn BarMenuList(entries: Vec<ds::MenuEntry<BarPick>>) -> Element {
    let link = use_context::<PopupLink>();
    let mut picked = use_hook(|| CopyValue::new(false));
    let mut hovered = use_hook(|| CopyValue::new(None::<usize>));
    let send = {
        let reports = link.reports.clone();
        move |report: PopupReport| {
            // The bar may be gone first; then there is nobody to tell.
            let _ = reports.send(report);
        }
    };
    let on_pick = {
        let send = send.clone();
        move |pick: BarPick| {
            picked.set(true);
            send(PopupReport::Picked(pick));
        }
    };
    let on_close = {
        let send = send.clone();
        move |()| {
            if !picked() {
                send(PopupReport::Dismissed);
            }
        }
    };
    let on_hover = {
        let send = send.clone();
        move |index: Option<usize>| {
            hovered.set(index);
            send(PopupReport::Hovered(index));
        }
    };
    let on_release = {
        let entries = entries.clone();
        move |_: Press| {
            let over = hovered().and_then(|index| choice(&entries, index));
            send(PopupReport::Released(over));
        }
    };
    rsx! {
        Menu {
            kind: MenuKind::Dropdown,
            anchor: corner(),
            entries,
            onpick: on_pick,
            onclose: on_close,
            on_hover,
            on_release,
            entrance: MenuEntrance::Instant,
        }
    }
}

/// The control center (`module: None`) or `module`'s own dropdown, on a quire `Popover` card
/// at the popup's corner: open at once, as a bar menu (a hover switch swaps it in place).
/// Escape on the grid, an outside press, or a pick that leaves the panel reports a dismissal.
#[component]
fn PanelPopover(module: Option<ControlCenterModule>) -> Element {
    let link = use_context::<PopupLink>();
    let reports = link.reports.clone();
    let pages = link.pages.clone();
    let output = link.output.clone();
    let dismissed = EventHandler::new(move |()| {
        // The bar may be gone first; then there is nobody to tell.
        let _ = reports.send(PopupReport::Dismissed);
    });
    let content = match module {
        None => rsx! {
            ControlCenter {
                output,
                on_close: dismissed,
                on_page: move |page: CcPage| {
                    let _ = pages.send(page);
                },
            }
        },
        Some(module) => rsx! {
            ModuleDropdown { module, output, on_close: dismissed }
        },
    };
    rsx! {
        Popover {
            anchor: corner(),
            placement: Placement::new(Side::Bottom, Align::Start),
            gap: GAP,
            elevation: Elevation::Pop,
            onclose: dismissed,
            {content}
        }
    }
}
