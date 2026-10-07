//! `IdleDim`'s input: what the caller wants of the overlay.

/// What the caller wants: the full-brightness screen, or the overlay dimmed to its level. Two
/// states, not a `bool` (`CONVENTIONS.md#4-types`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum IdleDimPhase {
    /// Nothing drawn: full brightness.
    #[default]
    Awake,
    /// Dimmed to `level`.
    Dimmed,
}
