//! Keys and two-state words for toplevels and workspaces. Protocol objects never leave the
//! thread: the thread gives each handle a key and publishes keys.

use serde::{Deserialize, Serialize};

/// A workspace, by the key the thread gave its handle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct WorkspaceKey(pub u64);

/// A workspace group (one per output, or shared), by the key the thread gave its handle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct GroupKey(pub u64);

/// An output by its connector name (`DP-1`), which is what both connections agree on:
/// shell-host's `OutputInfo::name` and cctk's `wl_output` name.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct OutputName(pub String);

/// Whether a toplevel or a workspace is the active one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Activity {
    Active,
    #[default]
    Inactive,
}

/// Whether a workspace wants attention.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Urgency {
    Urgent,
    #[default]
    Calm,
}

/// Whether a workspace is pinned (COSMIC keeps a pinned workspace when it empties).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Pinning {
    Pinned,
    #[default]
    Unpinned,
}
