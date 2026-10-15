//! The session's command line: `sill switcher next|prev|cancel|commit`, `sill lock [now|status]`,
//! `sill polkit status` and `sill idle status`, and their pure mappings to requests.

use clap::{Subcommand, ValueEnum};
use sill_ipc::act::{ActRequest, SwitcherRequest, WorkspaceRequest};
use sill_ipc::request::Request;
use sill_ipc::status::StatusRequest;

/// `sill switcher ...`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Subcommand)]
pub enum SwitcherAction {
    /// The chord (Cmd+Tab): arm the switcher, or move the selection on.
    Next,
    /// The reverse chord (Cmd+Shift+Tab).
    Prev,
    /// Close it and switch to nothing.
    Cancel,
    /// Switch to the selection and close it.
    Commit,
}

/// `sill workspace ...`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Subcommand)]
pub enum WorkspaceAction {
    /// Go to the workspace before the active one.
    Prev,
    /// Go to the workspace after the active one.
    Next,
    /// Show every window and workspace at once (Mission Control; fails on a session that is not ours).
    Overview,
}

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

/// The request `sill switcher ACTION` sends.
pub fn switcher_request(action: SwitcherAction) -> Request {
    Request::Act(ActRequest::Switcher(match action {
        SwitcherAction::Next => SwitcherRequest::Next,
        SwitcherAction::Prev => SwitcherRequest::Prev,
        SwitcherAction::Cancel => SwitcherRequest::Cancel,
        SwitcherAction::Commit => SwitcherRequest::Commit,
    }))
}

/// The request `sill workspace ACTION` sends.
pub fn workspace_request(action: WorkspaceAction) -> Request {
    Request::Act(ActRequest::Workspace(match action {
        WorkspaceAction::Prev => WorkspaceRequest::Prev,
        WorkspaceAction::Next => WorkspaceRequest::Next,
        WorkspaceAction::Overview => WorkspaceRequest::Overview,
    }))
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
