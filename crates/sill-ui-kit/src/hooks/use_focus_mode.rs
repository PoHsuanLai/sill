//! `use_focus_mode`: the focus mode in effect (`notifications.dnd`, read through the daemon's
//! `crate::links::FocusLink`).

use dioxus::prelude::{ReadSignal, use_context};
use shell_host::use_watch;
use sill_settings::OnOff;

use crate::links::FocusLink;

/// The focus mode in effect. Flipping it is the `ToggleFocus` action (`use_entrance`).
///
/// # Panics
/// When the surface's `Root` was built without `.context(FocusLink)`: a daemon wiring bug.
pub fn use_focus_mode() -> ReadSignal<OnOff> {
    use_watch(use_context::<FocusLink>().mode())
}
