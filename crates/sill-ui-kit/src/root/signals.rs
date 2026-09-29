//! Bridges shell-host's raw seat modality and surface scale into quire's `ds::HostSignals`, the
//! root context `Ds` reads to stamp `data-modality` (and so decide whether the focus ring shows)
//! and to size the pixel tokens. shell-host cannot name quire's `ds` crate
//! (`scripts/check-boundary.sh`), so every surface's own root wraps its `Ds` in this component,
//! which reads shell-host's `use_input_modality()` (one value per seat, shared by every surface)
//! and `use_surface_scale()` and republishes them as `ds::HostSignals` with a `use_effect`
//! (shell-host README, "Consuming: input modality (focus rings)"). The seat's window activity is
//! not reported to a surface, so the window is always active.
//!
//! A headless render with no `SurfaceHandle` (a `ds_native::Harness` test, `use_ds_scale`'s same
//! fallback) has no seat to read, so the bridge renders `children` unchanged: `Ds` then reads
//! whatever `HostSignals` the harness itself provides at its root (its own seat, driven by its
//! `key`/`click`), or falls back to `Pointer` with none at all.

use dioxus::prelude::*;
use ds::{Activity, HostSignals, InputModality as DsModality, Scale};
use shell_host::{InputModality as SeatModality, SurfaceHandle};

/// shell-host's raw modality, as quire's own.
fn to_ds(modality: SeatModality) -> DsModality {
    match modality {
        SeatModality::Pointer => DsModality::Pointer,
        SeatModality::Keyboard => DsModality::Keyboard,
    }
}

/// Wraps `children` (a surface's `Ds` tree) with `ds::HostSignals`, kept live from shell-host's
/// per-seat [`shell_host::use_input_modality`] and the surface's [`shell_host::use_surface_scale`]
/// on a real surface. Whether the handle exists never changes over a component's life
/// (`use_ds_scale`'s doc), so the hook below runs in the same order on every render.
#[component]
pub fn HostSignalsBridge(children: Element) -> Element {
    let hosted = use_hook(try_consume_context::<SurfaceHandle>);
    if hosted.is_none() {
        return rsx! { {children} };
    }
    let modality = shell_host::use_input_modality();
    let scale = shell_host::use_surface_scale();
    let signals = use_context_provider(|| HostSignals {
        modality: Signal::new(to_ds(*modality.peek())),
        scale: Signal::new(Scale(scale.peek().0)),
        activity: Signal::new(Activity::Active),
    });
    use_effect(move || {
        let (mut provided, next) = (signals.modality, to_ds(modality()));
        if *provided.peek() != next {
            provided.set(next);
        }
    });
    use_effect(move || {
        let (mut provided, next) = (signals.scale, Scale(scale().0));
        if *provided.peek() != next {
            provided.set(next);
        }
    });
    rsx! { {children} }
}
