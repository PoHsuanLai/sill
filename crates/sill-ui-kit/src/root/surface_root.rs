//! The root every layer surface's document starts with: quire's `Ds` given the desktop's
//! appearance, the look of the active workspace on the surface's output, and the tint and
//! material stack the settings choose. The device scale and the seat's input modality reach it
//! as `HostSignals` (`mount`). A surface supplies only what is its own: its material, its chrome
//! and its extent.

use dioxus::prelude::*;
use ds::prelude::*;
use ds_style::appearance::blur::BlurState;
use shell_host::OutputInfo;

use crate::hooks::use_appearance;
use crate::look::use_active_look;
use crate::root::use_root_style;

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
    let environment = use_appearance()();
    let look = use_active_look(&output);
    let root = use_root_style();
    rsx! {
        Ds {
            sheet: Some(ds_shell::stylesheet()),
            appearance: environment.settings.appearance.appearance(),
            system: environment.system,
            look,
            material,
            chrome,
            extent,
            blur: BlurState::Available,
            tint_alpha: Some(environment.tint_alpha()),
            stack: Some(environment.material_stack()),
            surface: root.surface,
            user_style: root.user_style,
            {children}
        }
    }
}
