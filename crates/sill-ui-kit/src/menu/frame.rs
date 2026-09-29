//! The frame every popup document draws its `Ds` root in: an absolutely placed box at the
//! surface's origin whose padding is the popup's transparent room, so `PopupSize::FitContent`
//! measures the menu plus its drop shadow's room and nothing else (`style.css`).

use dioxus::prelude::*;

use super::look::PopupRoom;

/// The frame's stylesheet.
pub const POPUP_FRAME_CSS: &str = include_str!("style.css");

/// `children` (a `Ds` root) in the popup frame, `room` around the card and at most `widest`
/// wide in all (none: no cap). `style` is written on the frame beside them, for the custom
/// properties the root's menu reads.
#[component]
pub fn PopupFrame(
    room: PopupRoom,
    #[props(default)] widest: Option<i32>,
    #[props(default)] style: String,
    children: Element,
) -> Element {
    let cap = widest.map_or(String::new(), |px| format!(";max-width:{px}px"));
    rsx! {
        style { {POPUP_FRAME_CSS} }
        div {
            class: "sill-popup-frame",
            style: "{room.style()}{cap};{style}",
            {children}
        }
    }
}
