//! The dbusmenu popup's document: one transparent `Ds` root over one `Menu{Dropdown}` that opens
//! at once (a bar menu), in the popup frame ([`PopupFrame`]): shell-host fits the popup to it
//! (`PopupSize::FitContent`), and the frame lays the card in flow, so the popup is the card and
//! its 8 px inset, and a press reaches the menu.

use dioxus::prelude::*;
use ds::host::measure::Anchor;
use ds::prelude::*;

use sill_ui_kit::hooks::{use_appearance, use_user_style};
use sill_ui_kit::menu::{PopupFrame, PopupRoom};
use sill_ui_kit::root::{RootStyle, SchemeKeyed};

use super::menu_popup::BAR_MENU;

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
    entries: Vec<MenuItem<i32>>,
    onpick: EventHandler<i32>,
    onclose: EventHandler<()>,
) -> Element {
    let environment = use_appearance()();
    let root = RootStyle::popup(BAR_MENU, use_user_style());
    rsx! {
           PopupFrame { room: INSET, widest: Some(POPUP_WIDTH),
               SchemeKeyed {
    scheme: resolve(environment.settings.appearance.appearance(), Theme::System, environment.system).scheme.slug().to_owned(),
    Ds {
                   sheet: Some(ds_shell::stylesheet()),
                   extent: RootExtent::Popup,
                   appearance: environment.settings.appearance.appearance(),
                   system: environment.system,
                   material: Material::Popover,
                   surface: root.surface,
                   user_style: root.user_style,
                   Menu {
                       placement: MenuPlacement::Bar,
                       // The frame lays the card in flow: the anchor places nothing.
                       anchor: Anchor::Point(Point::default()),
                       items: entries,
                       onpick,
                       onclose,
                   }
               }
    }
           }
       }
}

#[cfg(test)]
#[path = "tray_menu_tests.rs"]
mod tests;
