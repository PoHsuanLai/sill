//! Q80's stand-in: the pane switcher between the module grid and a detail pane (design/13
//! §13.3.7 "Detail"). quire's `PaneSwitcher` (on `cc-parts`: `shown`, `root`, `detail`) owns the
//! slide, the leaving pane and the timer; until it lands the shown pane alone is drawn, remounted
//! under a new key on every page change, and its box plays quire's own `slide-r` or `slide-l`
//! keyframe from the control center's sheet (no keyframes of sill's). Switching is this file.

use dioxus::prelude::*;

use crate::surfaces::control_center::page::{CcPage, Slide};

/// `root` (the grid) or `detail` as `shown` says, entering by `slide`.
#[component]
pub(crate) fn PaneSwitcher(shown: CcPage, slide: Slide, root: Element, detail: Element) -> Element {
    let key = format!("{shown:?}");
    let pane = match shown {
        CcPage::Modules => root,
        CcPage::Detail(_) => detail,
    };
    rsx! {
        for key in [key] {
            div { key: "{key}", class: "sill-cc-pane", "data-slide": slug(slide), {pane.clone()} }
        }
    }
}

/// `data-slide`: into a detail the pane comes in from the right (quire's `slide-r`, the
/// forward move), back to the grid from the left (`slide-l`).
fn slug(slide: Slide) -> &'static str {
    match slide {
        Slide::None => "none",
        Slide::ToDetail => "to-detail",
        Slide::ToModules => "to-modules",
    }
}
