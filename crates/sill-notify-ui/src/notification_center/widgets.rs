//! The widgets slot below the history (macOS's Notification Center widgets; design/20 §1.14):
//! Calendar, then Up Next, each a [`Widget`] on quire's card (a tile on the Popover, Medium).
//! Nothing adds, removes or orders them. The panel injects sill's widgets' stylesheets
//! (`widgets_css`).

use dioxus::prelude::*;
use sill_ui_kit::widgets::{CenterWidget, Widget};

/// The widgets under the history, in this order.
const CENTER: [CenterWidget; 2] = [CenterWidget::Calendar, CenterWidget::UpNext];

/// The widgets under the history.
#[component]
pub fn CenterWidgets() -> Element {
    rsx! {
        for kind in CENTER {
            Widget { key: "{kind:?}", kind }
        }
    }
}
