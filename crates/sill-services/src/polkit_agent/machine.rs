//! The agent's pure reducer: registration, the queue of requests, one helper run at a time for
//! the request on screen, and how each request completes. The live side performs the
//! [`PolkitEffect`]s and feeds back what happened.
//!
//! Rules (lane b fills [`reduce`] to these; `machine_tests.rs` is their table):
//!
//! - `Policy(Own)` while not registered: `Register`. `Policy(Off)` while registered:
//!   `Unregister`, every request completes `Cancelled`, registration `Passive`.
//! - `Registered` / `RegisterFailed(msg)` / `BusGone` set the registration (`Registered`,
//!   `Refused(msg)`, `NoBus`); a bus gone completes nothing (there is nobody to answer).
//! - `Begin` appends a request; with no user (no identity to ask for) it completes `Failed` at
//!   once. When it becomes the first, `StartHelper` for it (phase `Starting`).
//! - `Helper(cookie, line)` for the first request only: a prompt sets `Asking`; an error or info
//!   line is kept in `notes`; `Success` completes `Authorized`; `Failure` counts a try: under
//!   [`MAX_TRIES`] the request stays with `last = Wrong`, notes cleared, and `StartHelper` again;
//!   at [`MAX_TRIES`] it completes `Failed`. `HelperEnded` without either is a `Failure`.
//! - `Answered(cookie)` while `Asking`: `SendAnswer` and `Checking`; otherwise nothing.
//! - `Dismissed(cookie)` (the person) and `Cancel(cookie)` (polkitd): `StopHelper` if it is the
//!   first, and it completes `Cancelled`.
//! - Completing the first request starts the next one's helper.
//! - A cookie nobody holds changes nothing.

use crate::auth::{PamNote, UserName};

use super::helper::HelperLine;
use super::types::{AgentPolicy, AuthRequest, Cookie, LastTry, Polkit, PromptPhase, Registration};

/// How many helper runs one request gets (cosmic-osd and libpolkit-agent's clients: three).
pub const MAX_TRIES: u32 = 3;

/// Where a request came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Origin {
    /// polkitd's `BeginAuthentication`: its completion answers the method call.
    Bus,
    /// `sill debug polkit`: answered by the stand-in helper, completed to nobody.
    Fake,
}

/// One request in the queue.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Pending {
    pub request: AuthRequest,
    pub user: UserName,
    pub origin: Origin,
    pub phase: PromptPhase,
    pub last: LastTry,
    pub tries: u32,
    pub notes: Vec<PamNote>,
}

/// The whole state.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct PolkitState {
    pub policy: AgentPolicy,
    pub registration: Registration,
    /// The first is on screen.
    pub queue: Vec<Pending>,
}

/// One thing the live side learnt.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PolkitEvent {
    Policy(AgentPolicy),
    Registered,
    RegisterFailed(String),
    BusGone,
    /// `user` is the chosen identity's name; `None` when no identity could be chosen.
    Begin {
        request: AuthRequest,
        user: Option<UserName>,
        origin: Origin,
    },
    Cancel(Cookie),
    Dismissed(Cookie),
    Answered(Cookie),
    Helper(Cookie, HelperLine),
    HelperEnded(Cookie),
}

/// How a request ends.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Completion {
    /// The helper said `SUCCESS` (polkitd already knows: the helper told it).
    Authorized,
    /// The person or polkitd cancelled: `org.freedesktop.PolicyKit1.Error.Cancelled`.
    Cancelled,
    /// Out of tries, or nobody to ask for: `org.freedesktop.PolicyKit1.Error.Failed`.
    Failed,
}

/// What the live side must do.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PolkitEffect {
    Register,
    Unregister,
    StartHelper {
        cookie: Cookie,
        user: UserName,
    },
    /// Write the answer the live side holds for `cookie` to its helper.
    SendAnswer(Cookie),
    StopHelper(Cookie),
    Complete {
        cookie: Cookie,
        origin: Origin,
        completion: Completion,
    },
}

/// `state` after `event`.
pub fn reduce(state: PolkitState, event: PolkitEvent) -> (PolkitState, Vec<PolkitEffect>) {
    todo!("M11 fill: b-polkit ({state:?} {event:?})")
}

/// What the prompt follows: the registration, the first request as a view, how many wait.
pub fn polkit_of(state: &PolkitState) -> Polkit {
    todo!("M11 fill: b-polkit ({state:?})")
}

#[cfg(test)]
#[path = "machine_tests.rs"]
mod tests;
