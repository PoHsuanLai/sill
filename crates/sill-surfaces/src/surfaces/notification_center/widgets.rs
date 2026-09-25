//! The widgets slot below the history (macOS's Notification Center widgets; design/20 §1.14):
//! one [`Widget`] in the center host for each of `widgets.center`, in order, each a tile on the
//! Popover (the frame's). The panel injects every widget's stylesheet (`widgets_css`).

use dioxus::prelude::*;

use crate::hooks::use_settings;
use crate::widgets::{Widget, WidgetHost};

/// The widgets under the history, as `widgets.center` lists them.
#[component]
pub fn CenterWidgets() -> Element {
    let kinds = use_settings()().widgets.center;
    rsx! {
        for (at, kind) in kinds.into_iter().enumerate() {
            Widget { key: "{at}-{kind:?}", kind, host: WidgetHost::Center }
        }
    }
}
