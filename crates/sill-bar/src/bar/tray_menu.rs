//! The dbusmenu popup's document: one transparent `Ds` root over one `Menu{Dropdown}` that opens
//! at once (a bar menu), in the popup frame ([`PopupFrame`]): shell-host fits the popup to it
//! (`PopupSize::FitContent`), and the frame lays the card in flow, so the popup is the card and
//! its 8 px inset, and a press reaches the menu.

use dioxus::prelude::*;
use ds::{Anchor, Ds, Material, Menu, MenuEntrance, MenuEntry, MenuKind, Point, RootExtent};

use sill_ui_kit::hooks::use_appearance;
use sill_ui_kit::menu::{PopupFrame, PopupRoom};

/// The widest the popup is, card and inset together (design/04-COMPONENTS.md section 20,
/// Dropdown: "min 186, auto"; a longer label wraps).
pub(crate) const POPUP_WIDTH: i32 = 220;

/// The transparent inset around the card: the menu's 8 px from the popup's edge.
pub(crate) const INSET: PopupRoom = PopupRoom {
    side: 8,
    top: 8,
    bottom: 8,
};

/// `entries` as a `Menu{Dropdown}` in the frame; `onpick` hears an entry's id, `onclose` the menu
/// closing.
#[component]
pub(crate) fn TrayMenuDocument(
    entries: Vec<MenuEntry<i32>>,
    onpick: EventHandler<i32>,
    onclose: EventHandler<()>,
) -> Element {
    let environment = use_appearance()();
    rsx! {
        PopupFrame { room: INSET, widest: Some(POPUP_WIDTH),
            Ds {
                sheet: Some(ds_shell::stylesheet()),
                extent: RootExtent::Popup,
                appearance: environment.settings.appearance.appearance(),
                system: environment.system,
                material: Material::Popover,
                Menu {
                    kind: MenuKind::Dropdown,
                    // The frame lays the card in flow: the anchor places nothing.
                    anchor: Anchor::Point(Point::default()),
                    entries,
                    onpick,
                    onclose,
                    entrance: MenuEntrance::Instant,
                }
            }
        }
    }
}

#[cfg(test)]
#[path = "tray_menu_tests.rs"]
mod tests;
