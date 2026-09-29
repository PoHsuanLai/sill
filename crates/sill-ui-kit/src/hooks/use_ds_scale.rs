//! The surface's device scale for its `Ds { scale }` root (quire v0.1.4 pixel snapping): read
//! live from shell-host's `use_surface_scale` on a real surface, absent in a headless render
//! with no `SurfaceHandle` (the root then falls back to 1x). Whether the handle exists never
//! changes over a component's life, so the hooks below run in the same order on every render.

use dioxus::prelude::*;
use shell_host::{SurfaceHandle, use_surface_scale};

/// The scale to hand this surface's `Ds` root, in 120ths.
pub fn use_ds_scale() -> Option<ds::Scale> {
    let hosted = use_hook(try_consume_context::<SurfaceHandle>);
    hosted.map(|_| ds::Scale(use_surface_scale()().0))
}
