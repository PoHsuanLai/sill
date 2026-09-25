//! `use_focus_mode`: the focus mode in effect (`notifications.dnd`, read through the daemon's
//! `sill_settings::FocusLink`).

use dioxus::prelude::{ReadSignal, use_context};
use shell_host::use_watch;
use sill_settings::{FocusLink, FocusMode};

/// The focus mode in effect. Setting it is `use_context::<FocusLink>().set(mode)`.
///
/// # Panics
/// When the surface's `Root` was built without `.context(focus_link)`: a daemon wiring bug.
pub fn use_focus_mode() -> ReadSignal<FocusMode> {
    use_watch(use_context::<FocusLink>().mode())
}
