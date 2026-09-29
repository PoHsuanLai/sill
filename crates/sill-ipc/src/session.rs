//! M11's requests (sill FINDINGS "M11 freeze"): the app switcher's chord verbs, the lock and
//! polkit status, and the debug requests that fake an auth verdict, a polkit request and a
//! switcher open. The reports mirror sill-services' state (this crate may not depend on it).

use serde::{Deserialize, Serialize};

use crate::idle::ReportLockCause;

/// `sill switcher ...`: COSMIC's `WindowSwitcher` / `WindowSwitcherPrevious` shortcuts run
/// `next` / `prev` (`dist/cosmic`); `cancel` and `commit` are for scripts and for a compositor
/// that tells sill about the modifier's release some other way.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum SwitcherRequest {
    /// The chord (Cmd+Tab): arm the switcher, or move the selection on.
    Next,
    /// The reverse chord (Cmd+Shift+Tab).
    Prev,
    /// Close it and switch to nothing (Escape).
    Cancel,
    /// Switch to the selection and close it (the modifier's release).
    Commit,
}

/// `sill lock ...`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum LockRequest {
    /// Lock now: answered `Done` once asked.
    Now,
    /// What the lock screen is doing: answered with `Response::Lock`.
    Status,
}

/// `sill polkit ...`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum PolkitRequest {
    /// Whether sill's agent is registered and what it shows: answered with `Response::Polkit`.
    Status,
}

/// The lock screen as `sill lock status` prints it (`sill_services::Lock`'s mirror).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LockReport {
    pub phase: ReportLockPhase,
    /// What started the lock in place; `null` while unlocked.
    pub cause: Option<ReportLockCause>,
}

/// `sill_services::LockPhase`'s mirror.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReportLockPhase {
    Unlocked,
    Locking,
    Locked,
}

/// sill's polkit agent as `sill polkit status` prints it.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PolkitReport {
    pub registration: ReportRegistration,
    /// The action on screen, if any.
    pub showing: Option<String>,
    pub waiting: u32,
}

/// `sill_services::polkit_agent::Registration`'s mirror.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum ReportRegistration {
    Passive,
    NoBus,
    Registered,
    Refused(String),
}

/// A verdict made up by `sill debug auth accept|refuse`, fed to the lock screen's password
/// check as if PAM had answered. Refused unless the daemon was built with `debug`: an IPC
/// request must never unlock a real session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FakeAuth {
    Accept,
    Refuse,
}

/// A polkit request made up by `sill debug polkit`: the prompt shows it, a stand-in helper
/// answers it (it accepts `sill-fake`), and nobody waits on it.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FakePolkit {
    pub action_id: String,
    pub message: String,
}

/// The switcher opened on made-up apps by `sill debug switcher APP...`: shown at once (no show
/// delay), and Q, H and release act on no real window.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FakeSwitcher {
    /// Desktop ids, the current app first.
    pub apps: Vec<String>,
}
