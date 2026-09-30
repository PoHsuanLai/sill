//! The bar popup's own document: a `Ds` root in the popup frame ([`PopupFrame`]) holding one ds
//! `Menu{Dropdown}` (or, for the control center and a module's own item, a ds `Popover` around
//! `control_center::ControlCenter` or the module),
//! following the bar's view channel: a new view swaps the contents and, when its anchor moved,
//! repositions the popup (`xdg_popup.reposition`), none closes it. shell-host fits the popup to
//! the frame (`PopupSize::FitContent`), so the popup is the card and its room. The root is
//! `Material::Popover`, which paints nothing on its own box (quire's transparent popup root);
//! the menu card paints the material. The menu opens with no entrance (design/13 §13.3.2) and
//! reports the pointer's hover and release to the bar's tracking machine.

use dioxus::prelude::*;
use ds::{
    Align, Anchor, BlurState, Ds, Material, Menu, Placement, Point, Popover, Press, Px, RootExtent,
    Side,
};
use shell_host::{PopupPlacement, SurfaceOp, use_surface, use_watch};
use sill_model::ids::OutputName;
use sill_settings::ControlCenterModule;
use tokio::sync::{mpsc, watch};

use super::content::PopupBody;
use super::model::PopupView;
use super::pick::BarPick;
use super::plan::{PopupReport, choice};
use super::popup::{MAX_PX, room};
use crate::control_center::{ControlCenter, ModuleDropdown};
use sill_ui_kit::hooks::use_user_style;
use sill_ui_kit::menu::PopupFrame;
use sill_ui_kit::root::RootStyle;

/// The surface name of a bar menu (and a tray item's): the value of its `data-surface`.
pub(crate) const BAR_MENU: &str = "bar-menu";

/// The surface name of the control center and a module's own popup.
pub(crate) const CONTROL_CENTER: &str = "control-center";

/// The popup's link to its bar, provided as its root context.
#[derive(Debug, Clone)]
pub(crate) struct PopupLink {
    pub view: watch::Receiver<Option<PopupView>>,
    pub reports: mpsc::UnboundedSender<PopupReport>,
    /// The bar's output (the Display module's brightness target).
    pub output: Option<OutputName>,
}

/// The anchor a ds floating surface is given: the frame lays the card in flow (`menu/style.css`),
/// so it places nothing.
fn corner() -> Anchor {
    Anchor::Point(Point::default())
}

/// The control center's and the module dropdowns' stylesheet (lint-clean under
/// `Profile::Strict`, `tests/control_center_lint.rs`).
pub(crate) const CONTROL_CENTER_CSS: &str = include_str!("../control_center/style.css");

/// The gap `Popover` is asked for from [`corner`]'s point; the frame's flow ignores it.
const GAP: Px = Px(7.0);

/// The popup root. A popup is a `VirtualDom` of its own and inherits nothing from the bar, so
/// it provides quire's document host itself.
#[component]
pub(crate) fn BarPopup() -> Element {
    ds_native::provide_host();
    let link = use_context::<PopupLink>();
    let surface = use_surface();
    let user_style = use_user_style();
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
    // A text menu wraps at design/13 §13.3.3's maximum; a panel is as wide as its settings say.
    let (content, widest, name) = match current.body {
        PopupBody::Menu(entries) => (
            rsx! {
                BarMenuList { key: "{which:?}", entries }
            },
            Some(MAX_PX),
            BAR_MENU,
        ),
        PopupBody::ControlCenter(_) => (
            rsx! {
                PanelPopover { module: None }
            },
            None,
            CONTROL_CENTER,
        ),
        PopupBody::Module(module) => (
            rsx! {
                PanelPopover { module: Some(module) }
            },
            None,
            CONTROL_CENTER,
        ),
    };
    let root = RootStyle::popup(name, user_style);
    rsx! {
        style { {CONTROL_CENTER_CSS} }
        PopupFrame { room: room(), widest, style: look.metrics.style_attr(),
            Ds {
                sheet: Some(ds_shell::stylesheet()),
                extent: RootExtent::Popup,
                appearance: look.appearance,
                system: look.system,
                material: Material::Popover,
                blur: BlurState::Unavailable,
                tint_alpha: Some(look.tint),
                stack: Some(look.stack),
                surface: root.surface,
                user_style: root.user_style,
                {content}
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
fn BarMenuList(entries: Vec<ds::MenuItem<BarPick>>) -> Element {
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
            placement: ds::MenuPlacement::Bar,
            anchor: corner(),
            items: entries,
            onpick: on_pick,
            onclose: on_close,
            on_hover,
            on_release,
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
    let output = link.output.clone();
    let dismissed = EventHandler::new(move |()| {
        // The bar may be gone first; then there is nobody to tell.
        let _ = reports.send(PopupReport::Dismissed);
    });
    let content = match module {
        None => rsx! {
            ControlCenter { output, on_close: dismissed }
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
            onclose: dismissed,
            {content}
        }
    }
}
