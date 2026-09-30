//! The dim's render: quire's `IdleDim` alone in a transparent viewport root.

use dioxus::prelude::*;
use ds::{Ds, Material, RootChrome, RootExtent};
use ds_shell::IdleDim;
use shell_host::OutputInfo;

use super::io::use_dim;
use sill_ui_kit::hooks::use_appearance;

/// The dim on `output`.
pub(super) fn idle_dim(output: OutputInfo) -> Element {
    let _ = output;
    let scene = use_dim();
    let environment = use_appearance()();
    rsx! {
        Ds {
            sheet: Some(ds_shell::stylesheet()),
            appearance: environment.settings.appearance.appearance(),
            system: environment.system,
            material: Material::Sheet,
            chrome: Some(RootChrome::Transparent),
            extent: RootExtent::Viewport,
            IdleDim { level: scene.level, phase: scene.phase }
        }
    }
}
