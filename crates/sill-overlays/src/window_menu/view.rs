//! The window menu surface's render: a transparent viewport root with nothing in it. The menu is
//! the popup [`super::io::use_window_menu`] opens.

use dioxus::prelude::*;
use ds::prelude::*;
use shell_host::OutputInfo;

use super::io::use_window_menu;
use sill_ui_kit::hooks::use_appearance;
use sill_ui_kit::links::WindowMenuLink;
use sill_ui_kit::menu::use_popup_look;
use sill_ui_kit::root::use_root_style;

/// The window menu surface on `output`.
pub(super) fn window_menu(output: OutputInfo) -> Element {
    let link = use_context::<WindowMenuLink>();
    let look = use_popup_look();
    use_window_menu(&link, &output, look);
    let environment = use_appearance()();
    let root = use_root_style();
    rsx! {
        Ds {
            sheet: Some(sill_shell_kit::stylesheet()),
            appearance: environment.settings.appearance.appearance(),
            system: environment.system,
            material: Material::Sheet,
            chrome: Some(RootChrome::Transparent),
            extent: RootExtent::Viewport,
            surface: root.surface,
            styles: root.styles,
            custom_styles: root.custom_styles,
            div {}
        }
    }
}
