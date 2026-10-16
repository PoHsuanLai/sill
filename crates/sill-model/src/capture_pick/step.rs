//! The picker's state and its one transition function.

use super::ask::Ask;
use super::color::{ColorAsking, ColorOutcome, ColorReply, Eyedropper};
use super::permission::{AskedBy, PermissionAsking, PermissionOutcome};
use super::plan::Candidates;
use super::reply::{Picks, Reply, outcome_reply};
use crate::desktop::{Answer, Consent, ConsentId};

/// The most permission questions that wait behind the one on screen. The portal asks one at a
/// time and the compositor sends the next after an answer, so more than this is a runaway; the
/// portal's call is refused and the compositor's is dismissed without being remembered.
pub const WAITING_MAX: usize = 8;

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
    /// The permission questions waiting for it, in the order they came: from the portal and from
    /// the compositor alike, one sheet at a time.
    pub waiting: Vec<PermissionAsking>,
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
    /// The compositor asks whether a program may use a power.
    Consent(Consent),
    /// The compositor no longer needs the answer to this question.
    ConsentWithdrawn(ConsentId),
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
    /// A permission question arrived: on screen when the sheet is free, else it waits its turn.
    PermissionOpened(PermissionAsking),
    PermissionAnswered(AskId, PermissionOutcome),
    /// The compositor withdrew a question: it comes off the screen or the queue unanswered.
    PermissionWithdrawn(ConsentId),
    /// The portal backend left the bus: nobody waits for its questions.
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
    /// Tell the compositor what the person said to its question.
    ConsentReply(ConsentId, Answer),
}

/// Whether a request may be opened.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Admit {
    Yes,
    /// Another request is open.
    Busy,
}

/// Whether `Opened` or `ColorOpened` is taken now: one sheet of any kind at a time.
pub fn admits(state: &CapturePicker) -> Admit {
    match (&state.asking, &state.color, &state.permission) {
        (None, None, None) => Admit::Yes,
        (Some(_), _, _) | (_, Some(_), _) | (_, _, Some(_)) => Admit::Busy,
    }
}

/// Whether a permission question is taken now: on screen when the sheet is free, else it waits
/// behind the others, up to [`WAITING_MAX`].
pub fn permission_admits(state: &CapturePicker) -> Admit {
    match state.waiting.len() < WAITING_MAX {
        true => Admit::Yes,
        false => Admit::Busy,
    }
}

/// `state` with the first waiting question on screen, when nothing else is.
fn promote(state: CapturePicker) -> CapturePicker {
    if admits(&state) == Admit::Busy {
        return state;
    }
    let mut waiting = state.waiting.into_iter();
    let permission = waiting.next();
    CapturePicker {
        permission,
        waiting: waiting.collect(),
        ..state
    }
}

/// Whether the compositor's question `ask` is on screen or waiting.
pub fn holds_consent(state: &CapturePicker, ask: ConsentId) -> bool {
    let by = AskedBy::Compositor(ask);
    state
        .permission
        .iter()
        .chain(&state.waiting)
        .any(|held| held.by == by)
}

/// `state` with the permission question `asking` on screen, or waiting its turn. A question the
/// compositor repeats (it sends the open ones again when the shell restarts) is the one already
/// held, and a question that finds the line full is not taken.
fn queue(state: CapturePicker, asking: PermissionAsking) -> CapturePicker {
    if matches!(asking.by, AskedBy::Compositor(ask) if holds_consent(&state, ask))
        || permission_admits(&state) == Admit::Busy
    {
        return state;
    }
    let waiting = state.waiting.into_iter().chain(Some(asking)).collect();
    promote(CapturePicker { waiting, ..state })
}

/// `state` without the compositor's question `ask`, wherever it is; the next one takes the sheet.
fn withdraw(state: CapturePicker, ask: ConsentId) -> CapturePicker {
    let by = AskedBy::Compositor(ask);
    let permission = state.permission.filter(|asking| asking.by != by);
    let waiting = state
        .waiting
        .into_iter()
        .filter(|asking| asking.by != by)
        .collect();
    promote(CapturePicker {
        permission,
        waiting,
        ..state
    })
}

/// `state` after the portal backend left: nothing it asked is still wanted. The compositor's
/// questions stay.
fn portal_left(state: CapturePicker) -> CapturePicker {
    let compositors = |asking: &PermissionAsking| asking.by != AskedBy::Portal;
    let permission = state.permission.filter(compositors);
    let waiting = state.waiting.into_iter().filter(compositors).collect();
    promote(CapturePicker {
        asking: None,
        color: None,
        permission,
        waiting,
        ..state
    })
}

/// What the answer to `asking` is, for whoever asked.
fn reply_of(asking: &PermissionAsking, outcome: PermissionOutcome) -> CaptureOut {
    match asking.by {
        AskedBy::Portal => CaptureOut::PermissionReply(asking.id, outcome),
        AskedBy::Compositor(ask) => CaptureOut::ConsentReply(ask, outcome.answer()),
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
                    promote(CapturePicker {
                        asking: None,
                        ..state
                    }),
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
                promote(CapturePicker {
                    color: None,
                    ..state
                }),
                vec![CaptureOut::ColorReply(id, ColorReply::of(outcome))],
            ),
            Some(_) | None => (state, Vec::new()),
        },
        CaptureEvent::PermissionOpened(asking) => (queue(state, asking), Vec::new()),
        CaptureEvent::PermissionAnswered(id, outcome) => match &state.permission {
            Some(asking) if asking.id == id => {
                let reply = reply_of(asking, outcome);
                (
                    promote(CapturePicker {
                        permission: None,
                        ..state
                    }),
                    vec![reply],
                )
            }
            Some(_) | None => (state, Vec::new()),
        },
        CaptureEvent::PermissionWithdrawn(ask) => (withdraw(state, ask), Vec::new()),
        CaptureEvent::CallerGone => (portal_left(state), Vec::new()),
    }
}
