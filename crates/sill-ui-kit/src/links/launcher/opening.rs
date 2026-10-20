//! What a launcher opening carries beside the shared parts: with what already typed, for what,
//! and since when.

use std::time::Instant;

use sill_launcher::{LauncherMode, Scope};

use crate::links::Opening;

/// What the launcher is opened with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LauncherAsk {
    /// The query the field starts with (`sill launcher open --query`).
    pub query: String,
    /// What the panel is for: finding things, or talking to the companion. A change of it in
    /// place (the same serial) keeps the rows, the selection and the query.
    pub mode: LauncherMode,
    /// Which providers the panel starts on: every one, or one alone (the emoji grid).
    pub scope: Scope,
    /// When the request arrived, for `open_latency_ms`.
    pub requested: Instant,
}

/// One opening of the launcher. A new one remounts the panel's contents (the entrance plays, the
/// field is focused and the selection starts at the top).
pub type LauncherOpening = Opening<LauncherAsk>;
