//! M11's command line (FINDINGS "M11 freeze"): `sill switcher next|prev|cancel|commit`,
//! `sill lock [now|status]`, `sill polkit status`, `sill debug auth accept|refuse`, `sill debug
//! polkit`, `sill debug switcher APP...`, and their pure mappings to IPC requests.

use clap::{Args, Subcommand, ValueEnum};
use sill_ipc::{
    FakeAuth, FakePolkit, FakeSwitcher, IdleRequest, LockRequest, PolkitRequest, Request,
    SwitcherRequest,
};

/// `sill switcher ...`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Subcommand)]
pub enum SwitcherAction {
    /// The chord (COSMIC's `WindowSwitcher` shortcut).
    Next,
    /// The reverse chord (`WindowSwitcherPrevious`).
    Prev,
    /// Close with no switch.
    Cancel,
    /// Switch to the selection and close.
    Commit,
}

/// `sill lock [now|status]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum LockVerb {
    /// Lock the screen now with the configured locker.
    Now,
    /// Print what the lock screen is doing.
    Status,
}

/// `sill idle ...` (FINDINGS "sill idle fill").
#[derive(Debug, Clone, Copy, PartialEq, Eq, Subcommand)]
pub enum IdleAction {
    /// Who owns idle, where it has got to, and what holds it off.
    Status,
}

/// `sill polkit ...`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Subcommand)]
pub enum PolkitAction {
    /// Whether sill's agent is registered, and what it shows.
    Status,
}

/// `sill debug auth VERDICT`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum FakeAuthArg {
    Accept,
    Refuse,
}

/// `sill debug polkit [--action ID] [--message TEXT]`.
#[derive(Debug, Clone, PartialEq, Eq, Args)]
pub struct FakePolkitArgs {
    /// The action id the prompt names.
    #[arg(long = "action", default_value = "org.sill.debug.prompt")]
    pub action_id: String,
    /// The sentence the prompt shows.
    #[arg(long, default_value = "sill is testing its password prompt")]
    pub message: String,
}

/// The request `sill switcher ACTION` sends.
pub fn switcher_request(action: SwitcherAction) -> Request {
    Request::Switcher(match action {
        SwitcherAction::Next => SwitcherRequest::Next,
        SwitcherAction::Prev => SwitcherRequest::Prev,
        SwitcherAction::Cancel => SwitcherRequest::Cancel,
        SwitcherAction::Commit => SwitcherRequest::Commit,
    })
}

/// The request `sill lock VERB` sends.
pub fn lock_request(verb: LockVerb) -> Request {
    match verb {
        LockVerb::Now => Request::Lock,
        LockVerb::Status => Request::LockStatus(LockRequest::Status),
    }
}

/// The request `sill idle ACTION` sends.
pub fn idle_request(action: IdleAction) -> Request {
    match action {
        IdleAction::Status => Request::Idle(IdleRequest::Status),
    }
}

/// The request `sill polkit ACTION` sends.
pub fn polkit_request(action: PolkitAction) -> Request {
    match action {
        PolkitAction::Status => Request::Polkit(PolkitRequest::Status),
    }
}

/// `sill debug auth VERDICT`'s made-up verdict.
pub fn fake_auth(verdict: FakeAuthArg) -> FakeAuth {
    match verdict {
        FakeAuthArg::Accept => FakeAuth::Accept,
        FakeAuthArg::Refuse => FakeAuth::Refuse,
    }
}

/// `sill debug polkit`'s made-up request.
pub fn fake_polkit(args: &FakePolkitArgs) -> FakePolkit {
    FakePolkit {
        action_id: args.action_id.clone(),
        message: args.message.clone(),
    }
}

/// `sill debug switcher APP...`'s made-up apps.
pub fn fake_switcher(apps: &[String]) -> FakeSwitcher {
    FakeSwitcher {
        apps: apps.to_vec(),
    }
}
