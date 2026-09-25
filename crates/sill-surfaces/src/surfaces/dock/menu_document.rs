//! The context menu popup's document: a `Ds` root in a frame padded for the shadow room
//! (`menu_geometry`), holding one ds `Menu{Context}` anchored at the menu's bottom-left corner.
//! The root is `Material::Popover`, transparent on its own box; the menu card paints the
//! material and its drop, which the frame's room keeps inside the popup surface. The frame is in
//! flow at the viewport's size and the root fills it (quire CONSUMING §2's root-frame rule), so
//! a press reaches the menu.

use dioxus::prelude::*;
use ds::{Anchor, BlurState, Ds, Material, Menu, MenuKind, Point, Px};

use crate::surfaces::dock::menu_geometry::{frame_style, menu_point};
use crate::surfaces::dock::menu_popup::MenuLink;
use crate::surfaces::dock::pick::DockPick;

/// The popup's stylesheet: the frame its `Ds` root sits in.
pub(crate) const DOCK_POPUP_CSS: &str = include_str!("../../style/dock_popup.css");

/// The menu of the [`MenuLink`] root context: `on_pick` hears a pick, `on_close` the menu
/// closing (after a pick too).
#[component]
pub(crate) fn MenuDocument(on_pick: EventHandler<DockPick>, on_close: EventHandler<()>) -> Element {
    let ds_scale = crate::hooks::use_ds_scale();
    let link = use_context::<MenuLink>();
    let (x, y) = menu_point(link.size);
    let look = link.look;
    rsx! {
        style { {DOCK_POPUP_CSS} }
        div {
            class: "sill-dock-popup-frame",
            style: "{frame_style()};{look.metrics.style_attr()}",
            Ds {
                scale: ds_scale,
                appearance: look.appearance,
                system: look.system,
                material: Material::Popover,
                blur: BlurState::Unavailable,
                tint_alpha: Some(look.tint),
                stack: Some(look.stack),
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

#[cfg(test)]
#[path = "menu_document_tests.rs"]
mod tests;
