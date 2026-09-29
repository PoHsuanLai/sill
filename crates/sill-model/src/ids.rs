//! Identifiers more than one service shares. Protocol objects never leave the COSMIC Wayland
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

/// A DDC display, named by the i2c bus it answers on (`5` for `/dev/i2c-5`).
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default, Serialize, Deserialize,
)]
#[serde(transparent)]
pub struct DdcId(pub u16);
