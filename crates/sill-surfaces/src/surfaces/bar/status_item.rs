//! One icon status item: a ds `IconButton{Tool}` in a title slot, expanded while its menu is
//! open (design/20 §1.1 "`IconButton{Tool}` per status item").

use dioxus::prelude::*;
use ds::{Icon, IconButton, IconButtonVariant, Switch, use_rect};

use super::menus::BarMenu;
use super::session::MenuBar;
use super::title::TitleSlot;

/// The status item for `menu`, drawn as `icon` with `tooltip`.
#[component]
pub(crate) fn StatusIcon(menu: BarMenu, icon: Icon, tooltip: String) -> Element {
    let bar = use_context::<MenuBar>();
    let probe = use_rect();
    let expanded = if bar.is_open(menu) {
        Switch::On
    } else {
        Switch::Off
    };
    rsx! {
        TitleSlot { menu, probe,
            IconButton {
                variant: IconButtonVariant::Tool,
                icon,
                label: menu.label().to_owned(),
                tooltip: Some(tooltip),
                expanded: Some(expanded),
                onclick: move |_| {},
                mounted: move |event| probe.on_mounted(event),
            }
        }
    }
}
