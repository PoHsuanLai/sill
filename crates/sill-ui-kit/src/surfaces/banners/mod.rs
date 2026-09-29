//! Notification banners (design/20 §1.6, design/13 §13.3.6): one Overlay column per output,
//! spawned hidden and kept warm like the OSD, shown only on the output the [`crate::links::BannerLink`] names
//! while a card is up. Cards stack at the top right under the bar (or, per
//! `notifications.banner_position`, the bottom right), newest first, macOS-style.
//!
//! The motion and the card are quire's (`BannerStack`, `NotificationCard`): a card slides in from
//! the right, opens its body, actions and close button under the pointer, swipes away, and
//! slides out while the cards after it heal. sill keeps the life of each card: its hold
//! (`notifications.banner_hold_ms`), paused while hovered (`step.rs`, pure, tested); which
//! cards show (`stack.rs`) and how they follow the service over time (`deck.rs`); what each
//! decision tells the service (`commands.rs`); and the input and blur regions (`elements.rs`).
//! The card's words are `notification_card`, shared with the notification center.

mod commands;
mod deck;
mod deck_cards;
mod elements;
mod io;
mod layer;
mod stack;
mod step;
mod surface;
mod view;

pub use surface::Banners;
pub use view::{BANNER_VARS, BANNERS_CSS};
