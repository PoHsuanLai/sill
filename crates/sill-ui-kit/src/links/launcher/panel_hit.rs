//! Which part of the launcher's panel takes presses, measured by the panel and read by the
//! catcher.

/// The part of the panel that takes presses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum PanelHit {
    /// The whole panel: before the card has been measured, and while the actions menu is open
    /// (it hangs from a row and may reach below the card).
    #[default]
    Whole,
    /// The card alone, from the panel's top down `height` logical pixels.
    Card { height: i32 },
}
