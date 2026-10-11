//! The picker's state and its one transition function.

use super::ask::Ask;
use super::plan::Candidates;
use super::reply::{Picks, Reply, outcome_reply};

/// Whether the shell serves the picker.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum CaptureLink {
    /// Not told to serve yet.
    #[default]
    Passive,
    Serving,
    /// The name or the bus was not available; the text is for the log.
    Unavailable(String),
}

/// A request's number: the sheet answers by it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct AskId(pub u64);

/// The request on screen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Asking {
    pub id: AskId,
    pub ask: Ask,
}

/// What the service publishes.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CapturePicker {
    pub link: CaptureLink,
    /// What there is to share now, kept current while nothing is asked too.
    pub candidates: Candidates,
    pub asking: Option<Asking>,
}

/// What the service is told.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CaptureCommand {
    /// Claim the interface (the daemon sends it once; passive until then).
    Serve,
    /// The windows that exist now.
    Windows(Vec<super::plan::WindowCandidate>),
    /// The screens that exist now.
    Outputs(Vec<super::plan::OutputCandidate>),
    /// The person pressed Allow (with their picks) or cancelled.
    Answer(AskId, Outcome),
}

/// How the person ended the request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    Allow(Picks),
    Cancel,
}

/// One thing that happened.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CaptureEvent {
    Link(CaptureLink),
    Candidates(Candidates),
    Opened(Asking),
    Answered(AskId, Outcome),
    /// The caller left the bus: nobody is waiting.
    CallerGone,
}

/// What the service must then do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CaptureOut {
    /// Return this to the caller of the request.
    Reply(AskId, Reply),
}

/// Whether a request may be opened.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Admit {
    Yes,
    /// Another request is open.
    Busy,
}

/// Whether `Opened` is taken now.
pub fn admits(state: &CapturePicker) -> Admit {
    match state.asking {
        None => Admit::Yes,
        Some(_) => Admit::Busy,
    }
}

/// `state` after `event`, and what to do.
pub fn step(state: CapturePicker, event: CaptureEvent) -> (CapturePicker, Vec<CaptureOut>) {
    match event {
        CaptureEvent::Link(link) => (CapturePicker { link, ..state }, Vec::new()),
        CaptureEvent::Candidates(candidates) => (
            CapturePicker {
                candidates,
                ..state
            },
            Vec::new(),
        ),
        CaptureEvent::Opened(asking) => match admits(&state) {
            Admit::Yes => (
                CapturePicker {
                    asking: Some(asking),
                    ..state
                },
                Vec::new(),
            ),
            Admit::Busy => (state, Vec::new()),
        },
        CaptureEvent::Answered(id, outcome) => match &state.asking {
            Some(asking) if asking.id == id => {
                let reply = outcome_reply(&asking.ask, &state.candidates, &outcome);
                (
                    CapturePicker {
                        asking: None,
                        ..state
                    },
                    vec![CaptureOut::Reply(id, reply)],
                )
            }
            Some(_) | None => (state, Vec::new()),
        },
        CaptureEvent::CallerGone => (
            CapturePicker {
                asking: None,
                ..state
            },
            Vec::new(),
        ),
    }
}
