//! The idle service's published state.

use super::inhibit::{Inhibition, LogindIdle};
use super::policy::{DimLevel, IdleMode, PowerSource};
use serde::{Deserialize, Serialize};

/// What surfaces (the dim overlay) and `sill idle status` read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct Idle {
    pub mode: IdleMode,
    pub phase: IdlePhase,
    /// The dim overlay's wish: every output's dim layer shows `Dimmed`.
    pub dim: DimShown,
    pub power: PowerSource,
    pub inhibition: Inhibition,
    /// `org.freedesktop.ScreenSaver` inhibits held (for `sill idle status`).
    pub inhibits: u32,
    /// Whether logind's `BlockInhibited` names `idle`.
    pub logind: LogindIdle,
}

/// Where idleness has got to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IdlePhase {
    /// In use, or idle below the first threshold.
    #[default]
    Active,
    /// Dimmed, the warning before the displays go off.
    Dimmed,
    /// The displays are off (locked or not).
    DisplaysOff,
    /// sill asked logind to suspend and no wake has come yet.
    Suspending,
}

/// The dim overlay's state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum DimShown {
    #[default]
    Clear,
    Dimmed(DimLevel),
}
