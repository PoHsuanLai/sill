//! WorkspacePills: the bar's workspace indicator, Mission Control's Spaces bar as one segmented
//! group on the frame ground (design/30 section 2.10): a track with the pills inside it, the
//! selected workspace a raised segment with the current-item shadow, the rest quiet text.
//! Markup: `div.ds-ws-pills[role=group]` holding `button.ds-ws-pill[data-selected][aria-current]`.
//! Each pill reports its presses with the button (a click activates, a right-click opens the
//! Space menu); a caller that drags pills to reorder them hears each pill's `BarPointer` press
//! and release.

use crate::bar::pointer::BarPointer;
use dioxus::prelude::*;
use ds::base::press::Press;
use ds::base::vocab::Selection;
use ds::base::word::Word;
use ds::components::controls::press::{ActivationKeys, PressListeners};
use ds::root::common::Common;

/// The group: `children` are its `WorkspacePill`s (or the caller's wrappers around them);
/// `label` names it.
#[component]
pub fn WorkspacePills(
    label: String,
    #[props(default)] common: Common,
    children: Element,
) -> Element {
    let class = common.class("ds-ws-pills");
    let data = common.data_attributes();
    rsx! {
        div {
            id: common.id.clone(),
            class,
            role: "group",
            "aria-label": "{label}",
            ..data,
            {children}
        }
    }
}

/// One workspace. `current` marks the workspace on screen (`data-selected`, `aria-current`).
#[component]
pub fn WorkspacePill(
    label: String,
    #[props(default)] current: Selection,
    onclick: EventHandler<Press>,
    #[props(default)] pointer: BarPointer,
    #[props(default)] common: Common,
) -> Element {
    let listen = PressListeners::new(onclick);
    let class = common.class("ds-ws-pill");
    let data = common.data_attributes();
    rsx! {
        button {
            r#type: "button",
            id: common.id.clone(),
            class,
            "data-selected": current.slug(),
            "aria-current": current.aria_current(),
            onmounted: move |event| common.mounted(event),
            onpointerdown: {
                let pointer = pointer.clone();
                move |event| pointer.pressed(event)
            },
            onpointerup: {
                let pointer = pointer.clone();
                move |event| pointer.released(event)
            },
            onpointerenter: move |event| pointer.entered(event),
            onclick: move |event| listen.click(&event),
            onkeydown: move |event| listen.key_down(&event, ActivationKeys::ReturnAndSpace),
            oncontextmenu: move |event| listen.context_menu(&event),
            onmouseup: move |event| listen.mouse_up(&event),
            ..data,
            "{label}"
        }
    }
}
