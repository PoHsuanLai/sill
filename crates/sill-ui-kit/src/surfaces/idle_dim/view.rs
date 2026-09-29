//! The dim's render: quire's `IdleDim` alone in a transparent viewport root.

use dioxus::prelude::*;
use ds::{Ds, IdleDim, Material, RootChrome, RootExtent};
use shell_host::OutputInfo;

use super::io::use_dim;
use crate::hooks::{use_appearance, use_ds_scale};

/// The dim on `output`.
pub(super) fn idle_dim(output: OutputInfo) -> Element {
    let _ = output;
    let scene = use_dim();
    let environment = use_appearance()();
    let ds_scale = use_ds_scale();
    rsx! {
        Ds {
            scale: ds_scale,
            appearance: environment.settings.appearance.appearance(),
            system: environment.system,
            material: Material::Sheet,
            chrome: Some(RootChrome::Transparent),
            extent: RootExtent::Viewport,
            IdleDim { level: scene.level, phase: scene.phase }
        }
    }
}
