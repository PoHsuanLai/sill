//! The dock's smaller pieces (design/10-BEHAVIOUR-dock.md section 10.3.2): the running dot under a
//! tile, the label above it, and the optional reflective floor inside the pill. The shell lays out the tiles; these read the dock tokens (`--dock-dot`,
//! `--dock-dot-gap`, `--dock-floor`) its `DockMetrics` writes.

use dioxus::prelude::*;
use ds::base::vocab::Shown;
use ds::components::overlays::tooltip::{Hint, HintSide};
use ds::motion::hover_intent::HoverProfile;
use ds::root::common::Common;

/// The running dot: a `--dock-dot` (4 px) circle centred under its tile, its centre
/// `--dock-dot-gap` (3 px) below the tile's bottom edge (so it sits inside the 6 px padding), in the dock's ink; it fades in over `--t-quick`. Place
/// it inside the tile's box (a positioned element), and leave it out when the app has no
/// window.
#[component]
pub fn RunningDot(#[props(default)] common: Common) -> Element {
    let class = common.class("ds-running-dot");
    let data = common.data_attributes();
    rsx! {
        span {
            class,
            id: common.id.clone(),
            "aria-hidden": "true",
            onmounted: move |event| common.mounted(event),
            ..data,
        }
    }
}

/// The reflective floor: a soft light rising from the pill's floor, off unless `dock.floor` is
/// on (`--dock-floor`). Place it first inside the dock's root, under the tiles.
#[component]
pub fn DockFloor(#[props(default)] common: Common) -> Element {
    let class = common.class("ds-dock-floor");
    let data = common.data_attributes();
    rsx! {
        div {
            class,
            id: common.id.clone(),
            "aria-hidden": "true",
            onmounted: move |event| common.mounted(event),
            ..data,
        }
    }
}

/// The label above a dock tile (design/30 section 2.10): the tile's name in a small dark pill,
/// 6 above it, opened by the Label profile of the hover machine (100 ms cold, at once while the
/// hub is warm, gone the moment the pointer leaves) and faded in and out over `--t-quick`.
///
/// `shown` hands it to the caller: `None` follows the pointer, `Some` shows or hides it at once
/// (a dock that hides its label while a menu is open, a press is down or a drag is under way
/// runs its own machine). `common` goes on the label's surface.
#[component]
pub fn DockLabel(
    text: String,
    #[props(default)] shown: Option<Shown>,
    #[props(default)] common: Common,
    children: Element,
) -> Element {
    rsx! {
        Hint {
            text,
            shown,
            profile: HoverProfile::Label,
            side: HintSide::Above,
            root: "ds-dock-label",
            common,
            {children}
        }
    }
}
