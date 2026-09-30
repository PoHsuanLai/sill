//! Bridges shell-host's raw seat modality and surface scale into quire's `ds::HostSignals`, the
//! root context `Ds` reads to stamp `data-modality` (and so decide whether the focus ring shows)
//! and to size the pixel tokens. shell-host cannot name quire's `ds` crate
//! (`scripts/check-boundary.sh`), so sill republishes: [`use_host_signals`] reads shell-host's
//! `use_input_modality()` (one value per seat, shared by every surface) and `use_surface_scale()`
//! and provides them as `ds::HostSignals` with a `use_effect` (shell-host README, "Consuming:
//! input modality (focus rings)"). It runs once per document root: in `mount` and `mount_lock`
//! for a surface and in `PopupFrame` for a popup, so every `Ds` root has the same keyboard-only
//! focus ring and one scale, and no surface calls it by hand. The seat's window activity is not
//! reported to a surface, so the window is always active.
//!
//! A headless render with no `SurfaceHandle` (a `ds_harness::Harness` test) has no seat to read,
//! so the hook provides nothing: `Ds` then reads whatever `HostSignals` the harness itself
//! provides at its root (its own seat, driven by its `key`/`click`), or falls back to `Pointer`
//! and 1x with none at all. Whether the handle exists never changes over a component's life, so
//! the hooks run in the same order on every render.

use dioxus::prelude::*;
use ds::{Activity, HostSignals, InputModality as DsModality, Scale};
use shell_host::{InputModality as SeatModality, Scale as SeatScale, SurfaceHandle};

/// shell-host's raw modality, as quire's own.
fn to_ds(modality: SeatModality) -> DsModality {
    match modality {
        SeatModality::Pointer => DsModality::Pointer,
        SeatModality::Keyboard => DsModality::Keyboard,
    }
}

/// Provides `ds::HostSignals` to this component and everything under it, kept live from
/// shell-host's per-seat [`shell_host::use_input_modality`] and the surface's
/// [`shell_host::use_surface_scale`]. Nothing is provided on a component with no `SurfaceHandle`.
pub fn use_host_signals() {
    let hosted = use_hook(try_consume_context::<SurfaceHandle>);
    if hosted.is_some() {
        use_republished(
            shell_host::use_input_modality(),
            shell_host::use_surface_scale(),
        );
    }
}

/// The seat's `modality` and the surface's `scale`, provided as `ds::HostSignals`.
pub(super) fn use_republished(modality: ReadSignal<SeatModality>, scale: ReadSignal<SeatScale>) {
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
}

/// The scale `Ds` sizes this surface's pixel tokens by: the host's, or 1x with none. For what a
/// surface draws outside `Ds` at device pixels (its icon rasters).
pub fn use_host_scale() -> Scale {
    use_hook(try_consume_context::<HostSignals>).map_or(Scale::ONE, |signals| (signals.scale)())
}

#[cfg(test)]
#[path = "signals_tests.rs"]
mod tests;
