//! `IdleDim`: the pre-screen-off dim overlay (design/22-SETTINGS.md section 3.24; sill FINDINGS
//! "sill idle"). A full-screen scrim, black at `level` (`--scrim-idle`), never real
//! brightness, that ignores every pointer and key event: sill's idle service owns waking the
//! display, this draws the dim and nothing else. It fades in over `--t-idle-dim --e-out`
//! towards `level` and snaps to nothing the instant `phase` goes back to
//! [`IdleDimPhase::Awake`] — there is no exit animation to wait for
//! (`drive::use_idle_dim`; CONSUMING.md "Idle dim").
//!
//! **Where it goes.** Its own root, above every other surface: `Ds { extent:
//! RootExtent::Viewport, chrome: Some(RootChrome::Transparent), .. }`, with `IdleDim` as the
//! root's only child (CONSUMING.md "A document's frame must have a height": `Viewport` is what
//! gives an all-positioned root one).

use crate::idle_dim::drive::use_idle_dim;
use crate::idle_dim::model::IdleDimPhase;
use dioxus::prelude::*;
use ds::base::vocab::Percent;
use ds::root::common::Common;

/// The idle dim overlay. `level` is `idle.dim_level_pct` (10..90); `phase` is the caller's own
/// request, driven by sill's idle service.
#[component]
pub fn IdleDim(level: Percent, phase: IdleDimPhase, #[props(default)] common: Common) -> Element {
    let share = use_idle_dim(level, phase);
    let class = common.class("ds-idle-dim");
    let data = common.data_attributes();
    rsx! {
        div {
            class,
            id: common.id.clone(),
            "aria-hidden": "true",
            style: "opacity:{share.css()}",
            onmounted: move |event| common.mounted(event),
            ..data,
        }
    }
}
