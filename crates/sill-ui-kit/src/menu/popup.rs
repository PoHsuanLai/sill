//! The popup menu's document: a `Ds` root in a frame padded for the shadow room, holding one ds
//! `Menu{Context}` at `anchor`. The root is `Material::Popover`, transparent on its own box; the
//! menu card paints the material and its drop, which the frame's room keeps inside the popup
//! surface. The frame is in flow at the viewport's size and the root fills it, so a press
//! reaches the menu. Reads [`PopupLook`] and [`PopupRoom`] from the root context.

use dioxus::prelude::*;
use ds::{Anchor, BlurState, Ds, Material, Menu, MenuKind};

use super::look::{PopupLook, PopupRoom};
use super::model::MenuModel;
use crate::hooks::use_ds_scale;

/// The popup's stylesheet: the frame its `Ds` root sits in.
const CSS: &str = include_str!("style.css");

/// `model`'s menu at `anchor`: `on_pick` hears a pick, `on_close` the menu closing (after a
/// pick too).
#[component]
pub fn MenuPopup<Model: MenuModel>(
    model: Model,
    anchor: Anchor,
    on_pick: EventHandler<Model::Pick>,
    on_close: EventHandler<()>,
) -> Element {
    let scale = use_ds_scale();
    let look = use_context::<PopupLook>();
    let room = use_context::<PopupRoom>();
    rsx! {
        style { {CSS} }
        div {
            class: "sill-menu-popup-frame",
            style: "{room.style()};{look.metrics.style_attr()}",
            Ds {
                scale,
                appearance: look.appearance,
                system: look.system,
                material: Material::Popover,
                blur: BlurState::Unavailable,
                tint_alpha: Some(look.tint),
                stack: Some(look.stack),
                Menu {
                    kind: MenuKind::Context,
                    anchor,
                    entries: model.entries(),
                    onpick: on_pick,
                    onclose: on_close,
                }
            }
        }
    }
}
