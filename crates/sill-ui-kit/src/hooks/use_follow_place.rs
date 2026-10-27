//! `use_follow_place`: a layer surface re-applies its size and margin when the numbers they come
//! from change (a resolved token, an output's size). Only those two: a surface whose input and
//! blur regions follow what it draws (the launcher's panel, the banner column) must not have
//! them reset with the rest of its configuration, as `use_follow_layer` does.

use dioxus::prelude::*;
use shell_host::{LayerSize, Margin, SurfaceHandle};

/// Re-apply `size` and `margin` on this surface whenever they differ from the last ones applied.
pub fn use_follow_place(size: LayerSize, margin: Margin) {
    let surface = try_use_context::<SurfaceHandle>();
    let mut last = use_hook(|| CopyValue::new((size, margin)));
    if *last.peek() == (size, margin) {
        return;
    }
    if let Some(surface) = surface {
        surface.set_size(size);
        surface.set_margin(margin);
    }
    last.set((size, margin));
}
