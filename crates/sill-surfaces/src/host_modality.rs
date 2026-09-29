//! Bridges shell-host's raw seat modality into quire's `ds::HostModality`, the root context
//! `Ds` reads to stamp `data-modality` and so decide whether the focus ring shows. shell-host
//! cannot name quire's `ds` crate (`scripts/check-boundary.sh`), so every
//! surface's own root wraps its `Ds` in this component, which reads shell-host's
//! `use_input_modality()` (one value per seat, shared by every surface) and republishes it as
//! `ds::HostModality` with a `use_effect` (shell-host README, "Consuming: input modality (focus
//! rings)"). Replaces the bar popup's own `bar/modality.rs::ModalityFrame`, which only saw keys
//! that reached its own subtree; this bridge sees every key the seat delivers to any surface.
//!
//! A headless render with no `SurfaceHandle` (a `ds_native::Harness` test, `use_ds_scale`'s same
//! fallback) has no seat to read, so the bridge renders `children` unchanged: `Ds` then reads
//! whatever `HostModality` the harness itself provides at its root (its own seat, driven by its
//! `key`/`click`), or falls back to `Pointer` with none at all.

use dioxus::prelude::*;
use ds::{HostModality, InputModality as DsModality};
use shell_host::{InputModality as HostModalityKind, SurfaceHandle};

/// shell-host's raw modality, as quire's own.
fn to_ds(modality: HostModalityKind) -> DsModality {
    match modality {
        HostModalityKind::Pointer => DsModality::Pointer,
        HostModalityKind::Keyboard => DsModality::Keyboard,
    }
}

/// Wraps `children` (a surface's `Ds` tree) with `ds::HostModality`, kept live from shell-host's
/// per-seat [`shell_host::use_input_modality`] on a real surface. Whether the handle exists
/// never changes over a component's life (`use_ds_scale`'s doc), so the hook below runs in the
/// same order on every render.
#[component]
pub fn HostModalityBridge(children: Element) -> Element {
    let hosted = use_hook(try_consume_context::<SurfaceHandle>);
    if hosted.is_none() {
        return rsx! { {children} };
    }
    let host = shell_host::use_input_modality();
    let HostModality(mut provided) =
        use_context_provider(|| HostModality(Signal::new(to_ds(*host.peek()))));
    use_effect(move || {
        let next = to_ds(host());
        if *provided.peek() != next {
            provided.set(next);
        }
    });
    rsx! { {children} }
}
