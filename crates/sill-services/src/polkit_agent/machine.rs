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
use super::types::{
    AgentPolicy, AuthRequest, Cookie, Echo, LastTry, Polkit, PromptPhase, PromptView, Registration,
};

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
    match event {
        PolkitEvent::Policy(policy) => on_policy(state, policy),
        PolkitEvent::Registered => on_registered(state),
        PolkitEvent::RegisterFailed(message) => (
            PolkitState {
                registration: Registration::Refused(message),
                ..state
            },
            Vec::new(),
        ),
        PolkitEvent::BusGone => (
            PolkitState {
                registration: Registration::NoBus,
                ..state
            },
            Vec::new(),
        ),
        PolkitEvent::Begin {
            request,
            user,
            origin,
        } => begin(state, request, user, origin),
        PolkitEvent::Cancel(cookie) | PolkitEvent::Dismissed(cookie) => {
            complete(state, &cookie, Completion::Cancelled)
        }
        PolkitEvent::Answered(cookie) => answered(state, &cookie),
        PolkitEvent::Helper(cookie, line) => helper_said(state, &cookie, line),
        PolkitEvent::HelperEnded(cookie) => helper_said(state, &cookie, HelperLine::Failure),
    }
}

/// What the prompt follows: the registration, the first request as a view, how many wait.
pub fn polkit_of(state: &PolkitState) -> Polkit {
    Polkit {
        registration: state.registration.clone(),
        current: state.queue.first().map(|pending| PromptView {
            cookie: pending.request.cookie.clone(),
            action_id: pending.request.action_id.clone(),
            message: pending.request.message.clone(),
            icon: pending.request.icon.clone(),
            details: pending.request.details.clone(),
            user: pending.user.clone(),
            phase: pending.phase.clone(),
            last: pending.last,
            notes: pending.notes.clone(),
        }),
        waiting: u32::try_from(state.queue.len().saturating_sub(1)).unwrap_or(u32::MAX),
    }
}

fn on_policy(state: PolkitState, policy: AgentPolicy) -> (PolkitState, Vec<PolkitEffect>) {
    let state = PolkitState { policy, ..state };
    match (policy, &state.registration) {
        (AgentPolicy::Own, Registration::Registered) => (state, Vec::new()),
        (AgentPolicy::Own, _) => (state, vec![PolkitEffect::Register]),
        (AgentPolicy::Off, Registration::Registered) => unregister(state),
        (AgentPolicy::Off, _) => (
            PolkitState {
                registration: Registration::Passive,
                ..state
            },
            Vec::new(),
        ),
    }
}

fn on_registered(state: PolkitState) -> (PolkitState, Vec<PolkitEffect>) {
    let state = PolkitState {
        registration: Registration::Registered,
        ..state
    };
    match state.policy {
        AgentPolicy::Own => (state, Vec::new()),
        // Turned off while the registration was on its way.
        AgentPolicy::Off => unregister(state),
    }
}

/// Unregister, and cancel every request, the one on screen first.
fn unregister(state: PolkitState) -> (PolkitState, Vec<PolkitEffect>) {
    let stop = state
        .queue
        .first()
        .map(|first| PolkitEffect::StopHelper(first.request.cookie.clone()));
    let cancels = state
        .queue
        .into_iter()
        .map(|pending| PolkitEffect::Complete {
            cookie: pending.request.cookie,
            origin: pending.origin,
            completion: Completion::Cancelled,
        });
    let effects = [PolkitEffect::Unregister]
        .into_iter()
        .chain(stop)
        .chain(cancels)
        .collect();
    let state = PolkitState {
        policy: state.policy,
        registration: Registration::Passive,
        queue: Vec::new(),
    };
    (state, effects)
}

fn begin(
    state: PolkitState,
    request: AuthRequest,
    user: Option<UserName>,
    origin: Origin,
) -> (PolkitState, Vec<PolkitEffect>) {
    let Some(user) = user else {
        let effect = PolkitEffect::Complete {
            cookie: request.cookie,
            origin,
            completion: Completion::Failed,
        };
        return (state, vec![effect]);
    };
    let mut queue = state.queue;
    queue.push(Pending {
        request,
        user,
        origin,
        phase: PromptPhase::Starting,
        last: LastTry::None,
        tries: 0,
        notes: Vec::new(),
    });
    let effects = match queue.as_slice() {
        [only] => vec![start_for(only)],
        _ => Vec::new(),
    };
    (PolkitState { queue, ..state }, effects)
}

fn start_for(pending: &Pending) -> PolkitEffect {
    PolkitEffect::StartHelper {
        cookie: pending.request.cookie.clone(),
        user: pending.user.clone(),
    }
}

/// The first request, when it holds `cookie`.
fn first_is(state: &PolkitState, cookie: &Cookie) -> bool {
    state
        .queue
        .first()
        .is_some_and(|pending| pending.request.cookie == *cookie)
}

fn answered(state: PolkitState, cookie: &Cookie) -> (PolkitState, Vec<PolkitEffect>) {
    let asking = state
        .queue
        .first()
        .is_some_and(|p| matches!(p.phase, PromptPhase::Asking { .. }));
    if !first_is(&state, cookie) || !asking {
        return (state, Vec::new());
    }
    let state = with_first(state, |p| p.phase = PromptPhase::Checking);
    (state, vec![PolkitEffect::SendAnswer(cookie.clone())])
}

fn helper_said(
    state: PolkitState,
    cookie: &Cookie,
    line: HelperLine,
) -> (PolkitState, Vec<PolkitEffect>) {
    if !first_is(&state, cookie) {
        return (state, Vec::new());
    }
    match line {
        HelperLine::PromptHidden(prompt) => asking(state, prompt, Echo::Hidden),
        HelperLine::PromptShown(prompt) => asking(state, prompt, Echo::Shown),
        HelperLine::Error(text) => noted(state, PamNote::Error(text)),
        HelperLine::Info(text) => noted(state, PamNote::Info(text)),
        HelperLine::Success => complete(state, cookie, Completion::Authorized),
        HelperLine::Failure => failed_try(state, cookie),
    }
}

fn asking(state: PolkitState, prompt: String, echo: Echo) -> (PolkitState, Vec<PolkitEffect>) {
    let state = with_first(state, |p| p.phase = PromptPhase::Asking { prompt, echo });
    (state, Vec::new())
}

fn noted(state: PolkitState, note: PamNote) -> (PolkitState, Vec<PolkitEffect>) {
    (with_first(state, |p| p.notes.push(note)), Vec::new())
}

fn failed_try(state: PolkitState, cookie: &Cookie) -> (PolkitState, Vec<PolkitEffect>) {
    let tries = state.queue.first().map_or(0, |p| p.tries) + 1;
    if tries >= MAX_TRIES {
        return complete(state, cookie, Completion::Failed);
    }
    let state = with_first(state, |p| {
        p.tries = tries;
        p.last = LastTry::Wrong;
        p.phase = PromptPhase::Starting;
        p.notes.clear();
    });
    let effects = state.queue.first().map(start_for).into_iter().collect();
    (state, effects)
}

/// `state` with its first request changed by `change`.
fn with_first(state: PolkitState, change: impl FnOnce(&mut Pending)) -> PolkitState {
    let mut queue = state.queue;
    if let Some(first) = queue.first_mut() {
        change(first);
    }
    PolkitState { queue, ..state }
}

/// Take `cookie`'s request out with `completion`: its helper stops when it is the first and
/// the request was not finished by the helper itself, and the next one's starts.
fn complete(
    state: PolkitState,
    cookie: &Cookie,
    completion: Completion,
) -> (PolkitState, Vec<PolkitEffect>) {
    let Some(at) = state.queue.iter().position(|p| p.request.cookie == *cookie) else {
        return (state, Vec::new());
    };
    let mut queue = state.queue;
    let gone = queue.remove(at);
    let stop = (at == 0 && completion == Completion::Cancelled)
        .then(|| PolkitEffect::StopHelper(cookie.clone()));
    let done = PolkitEffect::Complete {
        cookie: cookie.clone(),
        origin: gone.origin,
        completion,
    };
    let next = (at == 0).then(|| queue.first().map(start_for)).flatten();
    let effects = stop.into_iter().chain([done]).chain(next).collect();
    (PolkitState { queue, ..state }, effects)
}

#[cfg(test)]
#[path = "machine_tests.rs"]
mod tests;
