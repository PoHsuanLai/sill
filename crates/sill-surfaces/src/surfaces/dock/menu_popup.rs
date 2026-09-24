//! The context menu popup's own document: a `Ds` root in a frame inset for the shadow room
//! (`menu_geometry`), holding one ds `Menu{Context}` anchored at the menu's bottom-left corner.
//! It reports a pick, a dismissal and its own drop to the dock, and closes itself once the
//! menu has closed. A popup is a `VirtualDom` of its own, so it provides quire's measurer
//! itself. The root is `Material::Popover`, transparent on its own box; the menu card paints
//! the material and its drop, which the frame's room keeps inside the popup surface.

use dioxus::prelude::*;
use ds::{Alpha, Anchor, Appearance, BlurState, Ds, Material, Menu, MenuEntry, MenuKind};
use ds::{Point, Px, SystemPrefs};
use shell_host::{Size, use_surface};
use tokio::sync::mpsc;

use crate::surfaces::dock::menu_geometry::{frame_style, menu_point};
use crate::surfaces::dock::pick::DockPick;

/// The popup's stylesheet: the frame its `Ds` root sits in.
pub(crate) const DOCK_POPUP_CSS: &str = include_str!("../../style/dock_popup.css");

/// How the popup's root resolves its scheme: the dock's own, handed over.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct PopupLook {
    pub appearance: Appearance,
    pub system: SystemPrefs,
    pub tint: Alpha,
}

/// What the popup tells the dock.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PopupReport {
    Picked(DockPick),
    Dismissed,
    /// The popup's document went away (closed by the compositor, or after a report).
    Gone,
}

/// The popup's root context.
#[derive(Debug, Clone)]
pub(crate) struct MenuLink {
    pub entries: Vec<MenuEntry<DockPick>>,
    pub size: Size,
    pub look: PopupLook,
    pub reports: mpsc::UnboundedSender<PopupReport>,
}

/// The popup root.
#[component]
pub(crate) fn DockMenuPopup() -> Element {
    ds_native::measure::provide();
    let link = use_context::<MenuLink>();
    let surface = use_surface();
    let gone = link.reports.clone();
    use_drop(move || {
        // The dock may be gone first; then nobody is left to tell.
        let _ = gone.send(PopupReport::Gone);
    });
    let mut picked = use_hook(|| CopyValue::new(false));
    let on_pick = {
        let reports = link.reports.clone();
        move |pick: DockPick| {
            picked.set(true);
            let _ = reports.send(PopupReport::Picked(pick));
        }
    };
    // ds `Menu` calls `onpick` and then `onclose`: a close reports a dismissal only when
    // nothing was picked, then closes the popup.
    let on_close = {
        let reports = link.reports.clone();
        let surface = surface.clone();
        move |()| {
            let reports = reports.clone();
            let surface = surface.clone();
            spawn(async move {
                if !picked() {
                    let _ = reports.send(PopupReport::Dismissed);
                }
                surface.close();
            });
        }
    };
    let (x, y) = menu_point(link.size);
    let look = link.look;
    rsx! {
        style { {DOCK_POPUP_CSS} }
        div { class: "sill-dock-popup-frame", style: frame_style(),
            Ds {
                appearance: look.appearance,
                system: look.system,
                material: Material::Popover,
                blur: BlurState::Unavailable,
                tint_alpha: Some(look.tint),
                Menu {
                    kind: MenuKind::Context,
                    anchor: Anchor::Point(Point { x: Px(x), y: Px(y) }),
                    entries: link.entries.clone(),
                    onpick: on_pick,
                    onclose: on_close,
                }
            }
        }
    }
}
