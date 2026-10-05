//! A corner's render: a transparent element taking the pointer over the whole square (no `Ds`,
//! no material, never painted), feeding the pointer's enter and leave to [`super::io`].

use dioxus::prelude::*;
use ds::prelude::AppStyle;
use shell_host::OutputInfo;

use super::io::use_hot_corner;
use sill_ui_kit::links::Corner;
use sill_ui_kit::root::use_root_style;

/// This surface's stylesheet: the one rule the transparent element needs (a bare `div` has no
/// height without one), lint-clean under `Profile::Strict` (`crates/sill/tests/lint.rs`).
pub const HOT_CORNERS_CSS: &str = include_str!("style.css");

/// `corner` of `output`.
pub(super) fn hot_corner(output: OutputInfo, corner: Corner) -> Element {
    let entering = use_hot_corner(output.id, corner);
    let leaving = entering;
    let surface = use_root_style().surface;
    rsx! {
        AppStyle { css: HOT_CORNERS_CSS }
        div {
            class: "sill-hot-corner",
            "data-surface": surface,
            onpointerenter: move |_| entering.enter(),
            onpointerleave: move |_| leaving.leave(),
        }
    }
}
