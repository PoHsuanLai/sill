//! The frame every popup document draws its `Ds` root in: an absolutely placed box at the
//! surface's origin whose padding is the popup's transparent room, so `PopupSize::FitContent`
//! measures the menu plus its drop shadow's room and nothing else (`style.css`). The root inside
//! passes `RootExtent::Popup`, so quire lays its card in flow.

use dioxus::prelude::*;
use ds::prelude::AppStyle;

use super::look::PopupRoom;
use crate::root::use_host_signals;

/// The frame's stylesheet.
pub const POPUP_FRAME_CSS: &str = include_str!("style.css");

/// The sheet of a frame that has room for submenus (`submenu.css`).
pub(crate) const POPUP_SUBMENU_CSS: &str = include_str!("submenu.css");

/// `children` (a `Ds` root) in the popup frame, `room` around the card and at most `widest`
/// wide in all (none: no cap). `submenu_room` is the transparent width, in px, left of the card
/// for the submenus a menu of the root opens beside it: quire places a submenu right of its parent
/// and falls back to the left, against the box of its overlay, which `submenu.css` widens by this
/// much on the left. The card stays at the frame's right, so a popup that hangs from its right
/// edge (a status menu) keeps its card where it is. `style` is written on the frame beside them, for the custom
/// properties the root's menu reads. The frame provides the seat's `HostSignals` to the root, so
/// a popup has the same focus ring and scale as a surface.
#[component]
pub fn PopupFrame(
    room: PopupRoom,
    #[props(default)] widest: Option<i32>,
    #[props(default)] submenu_room: i32,
    #[props(default)] style: String,
    children: Element,
) -> Element {
    use_host_signals();
    let cap = widest.map_or(String::new(), |px| format!(";max-width:{px}px"));
    rsx! {
        AppStyle { css: POPUP_FRAME_CSS }
        if submenu_room > 0 {
            AppStyle { css: POPUP_SUBMENU_CSS }
        }
        div {
            class: "sill-popup-frame",
            style: "{room.style()}{cap};--popup-submenu-room:{submenu_room}px;{style}",
            {children}
        }
    }
}
