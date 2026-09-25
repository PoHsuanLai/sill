//! Q80's stand-in: the pane switcher between the module grid and a detail pane (design/13
//! §13.3.7 "Detail"). quire's `PaneSwitcher` (on `cc-parts`) owns the slide and its timer; until
//! it lands the pane is remounted under a new key on every page change and its box plays quire's
//! own `slide-r` or `slide-l` keyframe from the control center's sheet (no keyframes of sill's).

use dioxus::prelude::*;

use crate::surfaces::control_center::page::{CcPage, Slide};

/// `children` as the pane `pane`, entering by `slide`.
#[component]
pub(crate) fn PaneSwitcher(pane: CcPage, slide: Slide, children: Element) -> Element {
    let key = format!("{pane:?}");
    rsx! {
        for key in [key] {
            div { key: "{key}", class: "sill-cc-pane", "data-slide": slug(slide), {children.clone()} }
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
