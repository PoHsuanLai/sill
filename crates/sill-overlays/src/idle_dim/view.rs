//! The dim's render: quire's `IdleDim` alone in a transparent viewport root.

use dioxus::prelude::*;
use ds::prelude::*;
use shell_host::OutputInfo;
use sill_shell_kit::idle_dim::IdleDim;

use super::io::use_dim;
use sill_ui_kit::hooks::use_appearance;
use sill_ui_kit::root::use_root_style;

/// The dim on `output`.
pub(super) fn idle_dim(output: OutputInfo) -> Element {
    let _ = output;
    let scene = use_dim();
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
            IdleDim { level: scene.level, phase: scene.phase }
        }
    }
}
