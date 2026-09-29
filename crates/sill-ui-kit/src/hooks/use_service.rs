//! `use_service`, `use_service_part` and `use_control`: any service in the root's [`Services`],
//! named by its type.

use crate::services::Member;
use dioxus::prelude::{ReadSignal, ReadableExt, use_memo};
use shell_host::use_watch;
use sill_services::Control;

use super::use_services;

/// Service `S`'s state, live.
pub fn use_service<S: Member>() -> ReadSignal<S::State> {
    use_watch(S::of(&use_services()).state.clone())
}

/// One part of service `S`'s state, live: a reader of `part` wakes only when that part changes,
/// not on every change of the rest (the workspaces beside the windows, the history beside the
/// banners).
pub fn use_service_part<S: Member, T: Clone + PartialEq + 'static>(
    part: fn(&S::State) -> T,
) -> ReadSignal<T> {
    let whole = use_service::<S>();
    ReadSignal::from(use_memo(move || part(&whole.read())))
}

/// Where service `S`'s commands go.
pub fn use_control<S: Member>() -> Control<S::Command> {
    S::of(&use_services()).control.clone()
}
