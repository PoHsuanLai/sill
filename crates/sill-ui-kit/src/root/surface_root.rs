//! The root every layer surface's document starts with: quire's `Ds` given the desktop's
//! appearance, the surface's device scale, the look of the active workspace on the surface's
//! output, and the tint and material stack the settings choose, inside the bridge that carries
//! the seat's input modality. A surface supplies only what is its own: its material, its chrome
//! and its extent.

use dioxus::prelude::*;
use ds::{BlurState, Ds, Material, RootChrome, RootExtent};
use shell_host::OutputInfo;

use super::signals::HostSignalsBridge;
use crate::hooks::{use_appearance, use_ds_scale};
use crate::look::use_active_look;

/// The surface on `output`, drawn as `material` (`chrome` and `extent` when the material's own
/// are not wanted) around `children`.
#[component]
pub fn SurfaceRoot(
    output: OutputInfo,
    material: Material,
    #[props(default)] chrome: Option<RootChrome>,
    #[props(default)] extent: RootExtent,
    children: Element,
) -> Element {
    let scale = use_ds_scale();
    let environment = use_appearance()();
    let look = use_active_look(&output);
    rsx! {
        HostSignalsBridge {
            Ds {
                scale,
                appearance: environment.settings.appearance.appearance(),
                system: environment.system,
                look,
                material,
                chrome,
                extent,
                blur: BlurState::Available,
                tint_alpha: Some(environment.tint_alpha()),
                stack: Some(environment.material_stack()),
                {children}
            }
        }
    }
}
