//! Two-state words for toplevels and workspaces.

use serde::{Deserialize, Serialize};

/// Whether a toplevel or a workspace is the active one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Activity {
    Active,
    #[default]
    Inactive,
}

/// Whether a workspace is pinned (COSMIC keeps a pinned workspace when it empties).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Pinning {
    Pinned,
    #[default]
    Unpinned,
}
