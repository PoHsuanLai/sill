//! Notification banners (design/20 §1.6, design/13 §13.3.6): one Overlay column per output,
//! spawned hidden and kept warm like the OSD, shown only on the output the [`sill_ui_kit::links::BannerLink`] names
//! while a card is up. Cards stack at the top right under the bar, newest first,
//! macOS-style.
//!
//! The motion and the card are quire's (`BannerStack`, `NotificationCard`): a card slides in from
//! the right, opens its body, actions and close button under the pointer, swipes away, and
//! slides out while the cards after it heal. sill keeps the life of each card: its hold
//! (a constant in `step.rs`), paused while hovered (`step.rs`, pure, tested); which
//! cards show (`stack.rs`) and how they follow the service over time (`deck.rs`); what each
//! decision tells the service (`commands.rs`); and the input and blur regions (`elements.rs`).
//! The card's words are `notification_card`, shared with the notification center.
//!
//! The column never takes the keyboard on appear. `sill focus-latest` asks for it through the
//! link, which names the banners when they appeared after the screenshot thumbnail; the newest
//! card then holds it (`deck_keys.rs`, pure) and its hold pauses, arrows move along the stack,
//! Escape, Delete and Backspace dismiss and hand it on, and every way out gives it back
//! (`keys.rs` is the keymap, `io.rs` sets the surface's `Exclusive` and `None`).

mod commands;
mod deck;
mod deck_cards;
mod deck_keys;
mod elements;
mod io;
mod keys;
mod layer;
mod stack;
mod step;
mod surface;
mod view;

pub use surface::Banners;
pub use view::{BANNER_VARS, BANNERS_CSS};
