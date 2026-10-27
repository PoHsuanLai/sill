//! One icon status item: a quire `MenuBarItem`, expanded while its menu is
//! open (design/20 §1.1), sized by the stylesheet's `--bar-status-box` and `--bar-status-glyph`.

use dioxus::prelude::*;
use ds::components::controls::button_model::ImagePosition;
use ds::host::measure::use_rect;
use ds::prelude::*;
use ds::root::common::Common;
use sill_shell_kit::prelude::*;

use super::menus::BarMenu;
use super::title::{shown_of, use_title_pointer};

/// The status item for `menu`, drawn as `icon` (a glyph, or a layered status glyph for the Wi-Fi,
/// battery, volume and Bluetooth items) with `tooltip`. The button draws its own pill
/// while expanded.
#[component]
pub(crate) fn StatusIcon(
    menu: BarMenu,
    #[props(into)] icon: IconSource,
    tooltip: String,
) -> Element {
    let probe = use_rect();
    let pointer = use_title_pointer(menu, probe);
    rsx! {
        MenuBarItem {
            common: Common { mounted: Some(EventHandler::new(move |event| probe.on_mounted(event))), ..Common::default() },
            image: ImagePosition::Only,
            icon,
            label: menu.label().to_owned(),
            title: Some(tooltip),
            shown: shown_of(menu),
            pointer,
            onclick: move |_| {},
        }
    }
}
