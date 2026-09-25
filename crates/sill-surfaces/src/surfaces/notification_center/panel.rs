//! The center's panel (lane c, FINDINGS "The M6 fill-in"): follows the [`super::CenterLink`] onto
//! the surface's visibility; a `Ds` root (Popover) holding the column at the right edge,
//! `notifications.center_width_px` wide, sliding in from the right (quire motion only); a
//! header with Clear All (`Button{Quiet}`); the history from `use_notification_history` as
//! `group_views` rows drawn with `NotificationCard { place: CardPlace::Center }`, each group
//! with its header, "N more" and Clear; [`super::CenterWidgets`] below; keys through
//! `step_keys`; a click outside the column closes it.
//!
//! Frozen stub: draws nothing (the daemon spawns the surface today, hidden).

use dioxus::prelude::*;
use shell_host::OutputInfo;

/// The center on `output`.
pub(super) fn center(output: OutputInfo) -> Element {
    let _ = output;
    rsx! {}
}
