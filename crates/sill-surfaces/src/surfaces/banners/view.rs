//! The banner column (lane b, FINDINGS "The M6 fill-in"): follows `use_notifications` and the
//! [`super::BannerLink`], maps its surface while this output shows cards and unmaps it once the
//! last has left, runs one `machine::step` per card, draws each with `NotificationCard`, and
//! sends what they decide (`commands.rs`).
//!
//! Frozen stub: draws nothing (the daemon spawns the surface today, hidden).

use dioxus::prelude::*;
use shell_host::OutputInfo;

/// The column on `output`.
pub(super) fn banners(output: OutputInfo) -> Element {
    let _ = output;
    rsx! {}
}
