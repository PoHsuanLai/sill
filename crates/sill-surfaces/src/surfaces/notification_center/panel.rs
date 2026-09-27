//! The center's root: one transparent `Ds` in the Popover material, as large as the output below
//! the bar (`RootExtent::Viewport`), holding the column (`column.rs`) and its stylesheet. The
//! column is the one painted part; the rest of the surface is the click-outside catcher. The
//! widgets under the history bring their own stylesheets (`widgets_css`), injected beside it.

use dioxus::prelude::*;
use ds::{BlurState, Ds, Material, RootExtent};
use shell_host::OutputInfo;
use sill_services::OutputName;

use super::column::Column;
use crate::hooks::{use_appearance, use_ds_scale, use_settings, use_workspaces};
use crate::surfaces::launcher::panel::active_look;
use crate::widgets::{provide_registry, widgets_css};

/// The center's stylesheet (lint-clean under `Profile::Strict`,
/// `tests/notification_center_lint.rs`).
pub const NOTIFICATION_CENTER_CSS: &str = include_str!("../../style/notification_center.css");

/// The center on `output`.
pub(super) fn center(output: OutputInfo) -> Element {
    provide_registry();
    let ds_scale = use_ds_scale();
    let file = use_settings()();
    let environment = use_appearance()();
    let on = output.name.clone().map(OutputName);
    let look = active_look(&use_workspaces()(), on.as_ref(), &file.spaces);
    rsx! {
        Ds {
            scale: ds_scale,
            appearance: environment.settings.appearance.appearance(),
            system: environment.system,
            look,
            material: Material::Popover,
            extent: RootExtent::Viewport,
            blur: BlurState::Available,
            tint_alpha: Some(environment.tint_alpha()),
            stack: Some(environment.material_stack()),
            style { {NOTIFICATION_CENTER_CSS} }
            style { {widgets_css()} }
            Column { output: output.id }
        }
    }
}
