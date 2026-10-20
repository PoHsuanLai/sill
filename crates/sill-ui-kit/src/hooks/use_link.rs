//! `use_link`: a surface reads a link the daemon provides, and says once when it is missing.

use dioxus::prelude::{CopyValue, ReadableExt, WritableExt, try_use_context, use_hook};

/// Whether the missing-link line was already printed by this hook instance.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Telling {
    Untold,
    Told,
}

/// The link of type `T`, or `None` when the root does not provide it. A missing link is a wire
/// the daemon was meant to give, so it is said once per hook instance on stderr; the caller
/// keeps its fallback.
pub fn use_link<T: Clone + 'static>() -> Option<T> {
    let link = try_use_context::<T>();
    tell_if_missing(&link);
    link
}

/// Say once per hook instance on stderr that the link of type `T` is not provided, when `link`
/// is `None`. For a hook that reads its link some other way than [`use_link`] (once, in
/// `use_hook`) and keeps its own fallback.
pub(crate) fn tell_if_missing<T: 'static>(link: &Option<T>) {
    let mut told = use_hook(|| CopyValue::new(Telling::Untold));
    if link.is_none() && *told.peek() == Telling::Untold {
        told.set(Telling::Told);
        eprintln!(
            "sill: {} is not provided here; this control does nothing",
            std::any::type_name::<T>()
        );
    }
}
