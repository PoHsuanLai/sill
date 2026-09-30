//! The popup menu's document: a `Ds` root in the popup frame ([`PopupFrame`]), holding one ds
//! `Menu{Context}`. The root is `Material::Popover`, transparent on its own box; the menu card
//! paints the material and its drop, which the frame's room keeps inside the popup surface. The
//! frame lays the card in flow, so `PopupSize::FitContent` measures it and a press reaches it.
//! Reads [`PopupLook`] and [`PopupRoom`] from the root context.

use dioxus::prelude::*;
use ds::{Anchor, BlurState, Ds, Material, Menu, MenuKind, Point, RootExtent};

use super::frame::PopupFrame;
use super::look::{PopupLook, PopupRoom};
use super::model::MenuModel;
use crate::hooks::use_user_style;
use crate::root::RootStyle;

/// `model`'s menu: `on_pick` hears a pick, `on_close` the menu closing (after a pick too).
/// `surface` names the document for a user stylesheet (`data-surface`), `dock-menu`.
#[component]
pub fn MenuPopup<Model: MenuModel>(
    model: Model,
    on_pick: EventHandler<Model::Pick>,
    on_close: EventHandler<()>,
    surface: &'static str,
) -> Element {
    let look = use_context::<PopupLook>();
    let root = RootStyle::popup(surface, use_user_style());
    let room = use_context::<PopupRoom>();
    rsx! {
        PopupFrame { room, style: look.metrics.style_attr(),
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
                Menu {
                    kind: MenuKind::Context,
                    // The frame lays the card in flow: the anchor places nothing.
                    anchor: Anchor::Point(Point::default()),
                    entries: model.entries(),
                    onpick: on_pick,
                    onclose: on_close,
                }
            }
        }
    }
}
