//! The wallpaper's one piece of state: which of its two image layers is in front.

use dioxus::prelude::*;

use super::crossfade::Layers;

/// The persistent A/B state ([`Layers`]) a `Ds` frame keeps the same way (`root/ds.rs`'s
/// `use_frame_layers`): held outside any signal, since the wallpaper already re-renders whenever
/// `use_service::<WallpaperService>` or `use_appearance` changes, and nothing else needs to read
/// it.
pub(super) fn use_crossfade(
    image: Option<String>,
) -> [(&'static str, Option<&'static str>, Option<String>); 2] {
    let mut layers = use_hook(|| CopyValue::new(Layers::starting_at(image.clone())));
    let next = layers.peek().clone().show(image);
    let rendered = next.render();
    layers.set(next);
    rendered
}
