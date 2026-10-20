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

/// The surface mapped once `visibility` says `Shown`, and never unmapped here: for a surface whose
/// exit plays first and ends in [`use_unmap_on_hidden`]. A repeat sets nothing. Nothing to map
/// without a host (a test, an SSR render).
pub fn use_map_when_shown(visibility: impl Fn() -> Visibility + 'static) {
    let surface = try_use_context::<SurfaceHandle>();
    let mut applied = use_hook(|| CopyValue::new(Visibility::Hidden));
    use_effect(move || {
        let now = visibility();
        if *applied.peek() == now {
            return;
        }
        applied.set(now);
        if now == Visibility::Shown
            && let Some(surface) = &surface
        {
            surface.set_visibility(Visibility::Shown);
        }
    });
}

/// The handler for a card's `on_hidden`: its exit has settled, so unmap the surface. Nothing to
/// unmap without a host (a test, an SSR render).
pub fn use_unmap_on_hidden() -> EventHandler<()> {
    let surface = try_use_context::<SurfaceHandle>();
    use_hook(|| {
        EventHandler::new(move |()| {
            if let Some(surface) = &surface {
                surface.set_visibility(Visibility::Hidden);
            }
        })
    })
}
