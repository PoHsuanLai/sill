use dioxus::prelude::*;
use shell_host::{SurfaceHandle, Visibility};

/// The surface's mapping following `visibility` (read inside an effect, so the signals it reads
/// drive it): mapped while it says `Shown`, unmapped while it says `Hidden`. A repeat sets
/// nothing. Nothing to map without a host (a test, an SSR render).
pub fn use_follow_visibility(visibility: impl Fn() -> Visibility + 'static) {
    let surface = try_use_context::<SurfaceHandle>();
    let mut applied = use_hook(|| CopyValue::new(None::<Visibility>));
    use_effect(move || {
        let now = visibility();
        if *applied.peek() == Some(now) {
            return;
        }
        applied.set(Some(now));
        if let Some(surface) = &surface {
            surface.set_visibility(now);
        }
    });
}
