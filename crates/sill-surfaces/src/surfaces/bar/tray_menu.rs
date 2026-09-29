//! The dbusmenu popup's document: one transparent `Ds` root over one `Menu{Dropdown}` that opens
//! at once (a bar menu). The root is the viewport (`RootExtent::Viewport`): a root holding only
//! the floating menu was 0 px tall, and with it the page, so a press never reached the menu.

use dioxus::prelude::*;
use ds::{
    Anchor, Ds, Material, Menu, MenuEntrance, MenuEntry, MenuKind, Point, Px, Rect as DsRect,
    RootExtent, Size as DsSize,
};

use crate::hooks::use_appearance;

/// The popup's guessed initial width (design/04-COMPONENTS.md section 20, Dropdown: "min 186,
/// auto"), before its content is measured — and, doubling as the inner `Menu`'s synthetic
/// anchor rect width, so the menu's right edge lands at the popup's own right edge (`tray.rs`).
pub(crate) const POPUP_WIDTH: i32 = 220;

/// `entries` as a `Menu{Dropdown}` across the popup's top; `onpick` hears an entry's id,
/// `onclose` the menu closing.
#[component]
pub(crate) fn TrayMenuDocument(
    entries: Vec<MenuEntry<i32>>,
    onpick: EventHandler<i32>,
    onclose: EventHandler<()>,
) -> Element {
    let ds_scale = crate::hooks::use_ds_scale();
    let environment = use_appearance()();
    rsx! {
        Ds {
            scale: ds_scale,
            appearance: environment.settings.appearance.appearance(),
            system: environment.system,
            material: Material::Popover,
            extent: RootExtent::Viewport,
            Menu {
                kind: MenuKind::Dropdown,
                anchor: Anchor::Rect(DsRect {
                    origin: Point::default(),
                    size: DsSize {
                        width: Px(POPUP_WIDTH as f32),
                        height: Px(0.0),
                    },
                }),
                entries,
                onpick,
                onclose,
                entrance: MenuEntrance::Instant,
            }
        }
    }
}

#[cfg(test)]
#[path = "tray_menu_tests.rs"]
mod tests;
