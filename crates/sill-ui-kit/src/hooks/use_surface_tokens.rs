//! `use_surface_tokens`: a surface's geometry tokens as the cascade resolves them, over quire's
//! `ds_blitz::use_resolved`, which reads them again after every resolved frame (so after the
//! person's styles or the look change) and re-renders only when they differ. Until the first frame
//! after the element mounts the value is the reader's with nothing resolved, the shipped one.

use dioxus::prelude::*;
use ds::prelude::Px;
use ds_blitz::{Lookup, ResolvedTokens, use_resolved};

/// A surface's tokens built from the resolved length of each custom property they name.
pub trait ReadTokens: Copy + PartialEq + 'static {
    /// The tokens from `length` (`None` for a property not resolved: the shipped value).
    fn read_tokens(length: &dyn Fn(&str) -> Option<Px>) -> Self;
}

/// The tokens one element of a surface sees.
#[derive(Debug, Clone, Copy)]
pub struct SurfaceTokens<T: 'static> {
    resolved: ResolvedTokens<T>,
}

/// The tokens for the calling component. Give [`SurfaceTokens::mounted`] to the `onmounted` of
/// an element inside the surface's `Ds` root.
pub fn use_surface_tokens<T: ReadTokens>() -> SurfaceTokens<T> {
    let resolved = use_resolved(|lookup: &Lookup| T::read_tokens(&|name| lookup.length(name)));
    SurfaceTokens { resolved }
}

impl<T: ReadTokens> SurfaceTokens<T> {
    /// Read the tokens of the element `event` names.
    pub fn mounted(self, event: MountedEvent) {
        self.resolved.mounted(event);
    }

    /// The tokens now: the shipped values until the first frame after mount.
    pub fn get(&self) -> T {
        self.resolved.value()().unwrap_or_else(|| T::read_tokens(&|_| None))
    }
}
