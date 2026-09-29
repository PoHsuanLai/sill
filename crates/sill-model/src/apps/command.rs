//! What the apps service hears and what it can be told.

/// What the apps service can be told.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppsCommand {
    /// Rescan now (`sill reload`).
    Rescan,
}
