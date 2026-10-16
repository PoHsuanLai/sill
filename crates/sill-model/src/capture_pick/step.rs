//! The picker's state and its one transition function.

use super::ask::Ask;
use super::color::{ColorAsking, ColorOutcome, ColorReply, Eyedropper};
use super::permission::{PermissionAsking, PermissionOutcome};
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
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
    /// The colour request on screen; open only while the eyedropper is drawn.
    pub color: Option<ColorAsking>,
    /// The permission question on screen.
    pub permission: Option<PermissionAsking>,
    pub eyedropper: Eyedropper,
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
    /// A surface draws the eyedropper (or stopped).
    Eyedropper(Eyedropper),
    /// The person clicked a colour with the eyedropper or cancelled.
    ColorAnswer(AskId, ColorOutcome),
    /// The person pressed a button of the permission sheet, or it closed.
    PermissionAnswer(AskId, PermissionOutcome),
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
    Eyedropper(Eyedropper),
    ColorOpened(ColorAsking),
    ColorAnswered(AskId, ColorOutcome),
    PermissionOpened(PermissionAsking),
    PermissionAnswered(AskId, PermissionOutcome),
    /// The caller left the bus: nobody is waiting.
    CallerGone,
}

/// What the service must then do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CaptureOut {
    /// Return this to the caller of the request.
    Reply(AskId, Reply),
    /// Return this to the caller of the colour request.
    ColorReply(AskId, ColorReply),
    /// Return this to the caller of the permission question.
    PermissionReply(AskId, PermissionOutcome),
}

/// Whether a request may be opened.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Admit {
    Yes,
    /// Another request is open.
    Busy,
}

/// Whether `Opened`, `ColorOpened` or `PermissionOpened` is taken now: one question of any kind
/// at a time.
pub fn admits(state: &CapturePicker) -> Admit {
    match (&state.asking, &state.color, &state.permission) {
        (None, None, None) => Admit::Yes,
        (Some(_), _, _) | (_, Some(_), _) | (_, _, Some(_)) => Admit::Busy,
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
        CaptureEvent::Eyedropper(eyedropper) => (
            CapturePicker {
                eyedropper,
                ..state
            },
            Vec::new(),
        ),
        CaptureEvent::ColorOpened(asking) => match (admits(&state), state.eyedropper) {
            (Admit::Busy, _) => (state, Vec::new()),
            (Admit::Yes, Eyedropper::Absent) => (
                state,
                vec![CaptureOut::ColorReply(asking.id, ColorReply::cancelled())],
            ),
            (Admit::Yes, Eyedropper::Present) => (
                CapturePicker {
                    color: Some(asking),
                    ..state
                },
                Vec::new(),
            ),
        },
        CaptureEvent::ColorAnswered(id, outcome) => match &state.color {
            Some(asking) if asking.id == id => (
                CapturePicker {
                    color: None,
                    ..state
                },
                vec![CaptureOut::ColorReply(id, ColorReply::of(outcome))],
            ),
            Some(_) | None => (state, Vec::new()),
        },
        CaptureEvent::PermissionOpened(asking) => match admits(&state) {
            Admit::Yes => (
                CapturePicker {
                    permission: Some(asking),
                    ..state
                },
                Vec::new(),
            ),
            Admit::Busy => (state, Vec::new()),
        },
        CaptureEvent::PermissionAnswered(id, outcome) => match &state.permission {
            Some(asking) if asking.id == id => (
                CapturePicker {
                    permission: None,
                    ..state
                },
                vec![CaptureOut::PermissionReply(id, outcome)],
            ),
            Some(_) | None => (state, Vec::new()),
        },
        CaptureEvent::CallerGone => (
            CapturePicker {
                asking: None,
                color: None,
                permission: None,
                ..state
            },
            Vec::new(),
        ),
    }
}
