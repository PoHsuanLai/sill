//! The center's root: one transparent `Ds` in the Popover material, as large as the output below
//! the bar (`RootExtent::Viewport`), holding the column (`column.rs`) and its stylesheet. The
//! column is the one painted part; the rest of the surface is the click-outside catcher. The
//! widgets under the history bring their own stylesheets (`widgets_css`), injected beside it.

use dioxus::prelude::*;
use ds::{Material, RootExtent};
use shell_host::OutputInfo;

use super::column::Column;
use crate::root::SurfaceRoot;
use crate::widgets::{provide_registry, widgets_css};

/// The center's stylesheet (lint-clean under `Profile::Strict`,
/// `tests/notification_center_lint.rs`).
pub const NOTIFICATION_CENTER_CSS: &str = include_str!("../../style/notification_center.css");

/// The center on `output`.
pub(super) fn center(output: OutputInfo) -> Element {
    provide_registry();
    rsx! {
        SurfaceRoot {
            output: output.clone(),
            material: Material::Popover,
            extent: RootExtent::Viewport,
            style { {NOTIFICATION_CENTER_CSS} }
            style { {widgets_css()} }
            Column { output: output.id }
        }
    }
}
