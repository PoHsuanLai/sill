//! The session's command line: `sill lock [now|status]`, `sill polkit status` and `sill idle
//! status`, and their pure mappings to requests.

use clap::{Subcommand, ValueEnum};
use sill_ipc::act::ActRequest;
use sill_ipc::request::Request;
use sill_ipc::status::StatusRequest;

/// `sill lock [VERB]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum LockVerb {
    /// Lock now.
    Now,
    /// What the lock screen is doing.
    Status,
}

/// `sill idle ...`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Subcommand)]
pub enum IdleAction {
    /// Who owns idle, where it has got to, and what holds it off.
    Status,
}

/// `sill polkit ...`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Subcommand)]
pub enum PolkitAction {
    /// Whether sill's agent is registered and what it shows.
    Status,
}

/// The request `sill lock VERB` sends.
pub fn lock_request(verb: LockVerb) -> Request {
    match verb {
        LockVerb::Now => Request::Act(ActRequest::Lock),
        LockVerb::Status => Request::Status(StatusRequest::Lock),
    }
}

/// The request `sill idle ACTION` sends.
pub fn idle_request(action: IdleAction) -> Request {
    match action {
        IdleAction::Status => Request::Status(StatusRequest::Idle),
    }
}

/// The request `sill polkit ACTION` sends.
pub fn polkit_request(action: PolkitAction) -> Request {
    match action {
        PolkitAction::Status => Request::Status(StatusRequest::Polkit),
    }
}
