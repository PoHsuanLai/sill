//! Whether a new surface is on screen at once.

/// A surface's state right after the daemon spawns it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Start {
    /// Mapped at once.
    Shown,
    /// Spawned warm and unmapped before its first paint; something else maps it.
    Hidden,
}
