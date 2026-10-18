//! What the auth service can be told.

use ds_core::word::Word;
use serde::{Deserialize, Serialize};

use super::Secret;

/// A verdict made up by `sill debug auth` (debug builds only: the daemon refuses it otherwise).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Word)]
#[serde(rename_all = "snake_case")]
pub enum FakeVerdict {
    Accept,
    Refuse,
}

/// What the auth service can be told.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthCommand {
    /// Check `secret` for this session's user (ignored while `Checking`).
    Submit(Secret),
    /// A new lock begins, or the last one ended: back to `Idle`, notes cleared. The failure count
    /// is kept until a success (it only counts: no lockout).
    Reset,
    /// Feed a verdict as if a check had returned it, without PAM (`sill debug auth`).
    Fake(FakeVerdict),
    /// The lock grace opened the lock: `Accepted` with no check, only while a lock is up and
    /// nothing is being checked. Only the idle service can send it: a [`crate::idle::GraceTicket`]
    /// is minted by the grace rule alone.
    Grace(crate::idle::GraceTicket),
}
