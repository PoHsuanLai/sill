//! Notification banners (design/20 §1.6, design/13 §13.3.6): one Overlay column per output,
//! spawned hidden and kept warm like the OSD, shown only on the output the [`BannerLink`] names
//! while a card is up. Cards stack at the top right under the bar (or, per
//! `notifications.banner_position`, the bottom right), newest on top, macOS-style.
//!
//! A card enters sliding in from the right; its hold (`notifications.banner_hold_ms`) pauses
//! while hovered, and hovering expands it (six body lines, the action buttons, a close button);
//! a click on the body invokes `default`; a swipe right or a drag past the threshold dismisses
//! it. The life of one card is `machine.rs` (pure, tested); which cards show is `stack.rs`; what
//! each decision tells the service is `commands.rs`; the input and blur region is `regions.rs`.
//! The card itself is `notification_card`, shared with the notification center.

pub mod commands;
pub mod layer;
pub mod machine;
pub mod placement;
pub mod regions;
pub mod stack;

mod view;

pub use commands::{action_command, leave_commands};
pub use layer::{CARD_RADIUS, ROOM, banners_layer};
pub use machine::{
    Banner, BannerIn, BannerOut, BannerParams, Deadline, Leave, Persistence, Remaining, step,
};
pub use placement::{BannerLink, BannerOutput};
pub use regions::{CardBox, card_regions};
pub use stack::{Card, stack};

use dioxus::prelude::*;
use shell_host::OutputInfo;

/// The banner column on `output`. Needs a [`BannerLink`] root context beside the daemon's usual
/// ones (the daemon's `surface_root` gives every surface one).
#[component]
pub fn Banners(output: OutputInfo) -> Element {
    view::banners(output)
}
