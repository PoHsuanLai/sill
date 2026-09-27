//! sill's own idle on the wire (sill FINDINGS "sill idle (Q420 B)" Q446, "sill idle fill"):
//! `sill idle status`, and the lock's cause that `sill lock status` reports. The reports mirror
//! sill-services' state (this crate may not depend on it).

use serde::{Deserialize, Serialize};

/// `sill idle ...`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum IdleRequest {
    /// Who owns idle, where it has got to, and what holds it off: answered with
    /// `Response::Idle`.
    Status,
}

/// `sill_services::locker::LockCause`'s mirror.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReportLockCause {
    IdleScreenOff,
    Action,
    Logind,
    Sleep,
    Restored,
    Adopted,
}

/// sill's idle as `sill idle status` prints it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct IdleReport {
    pub mode: ReportIdleMode,
    pub phase: ReportIdlePhase,
    pub dim: ReportDim,
    pub power: ReportPower,
    /// Whether an inhibitor sill honours holds idle off.
    pub inhibition: ReportInhibition,
    /// `org.freedesktop.ScreenSaver` inhibits held.
    pub inhibits: u32,
    /// Whether logind's `BlockInhibited` names `idle`.
    pub logind: ReportInhibition,
}

/// `sill_services::idle::IdleMode`'s mirror.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum ReportIdleMode {
    Own,
    Off(ReportOffReason),
}

/// `sill_services::idle::OffReason`'s mirror.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReportOffReason {
    NoPolicy,
    CosmicSession,
    Guest,
    CosmicIdleRunning,
}

/// `sill_services::idle::IdlePhase`'s mirror.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReportIdlePhase {
    Active,
    Dimmed,
    DisplaysOff,
    Suspending,
}

/// `sill_services::idle::DimShown`'s mirror: the level is the brightness kept, in percent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum ReportDim {
    Clear,
    Dimmed(u8),
}

/// AC or battery.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReportPower {
    Ac,
    Battery,
}

/// Held off or free.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReportInhibition {
    Free,
    Held,
}
