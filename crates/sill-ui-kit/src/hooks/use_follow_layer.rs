//! `use_follow_layer`: a layer surface re-applies its size, exclusive zone and regions when a
//! setting changes the layer configuration it was spawned with (the daemon spawns each surface
//! once, with the settings of that moment).

use dioxus::prelude::*;
use shell_host::{LayerConfig, SurfaceHandle};

/// Re-apply `config` on this surface whenever it differs from the last one applied.
pub fn use_follow_layer(config: LayerConfig) {
    let surface = try_use_context::<SurfaceHandle>();
    let mut last = use_hook(|| CopyValue::new(config.clone()));
    if *last.peek() == config {
        return;
    }
    if let Some(surface) = surface {
        surface.set_size(config.size);
        surface.set_exclusive_zone(config.exclusive);
        surface.set_input_region(config.input.clone());
        surface.set_blur(config.blur.clone());
    }
    last.set(config);
}
