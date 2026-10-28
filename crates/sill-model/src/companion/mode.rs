//! The mode of the conversation the assistant's input box types into: Auto or Manual, which the
//! router decides each change by. The router holds the mode of every session; this is the shell's
//! view of the one the box is typing into, and the switch the person flips.
//!
//! Every ask opens a session of its own, so the mode travels with the turn: a turn carries the
//! mode the box shows once the router has told the shell one ([`ConversationMode::turn_mode`]).
//! Before that the box shows a placeholder and the turn carries none, so a new conversation starts
//! in what Settings says. A switch goes to the router as `Session.SetMode`; the box follows the
//! router's `ModeChanged`, whoever made the change, rather than its own request. A switch takes
//! effect from the next step, which the box says while a step is running. Pure.

use companion_wire::AnswerPhase;
use docket_permissions_core::Mode;
use prov::SessionId;

use super::command::CompanionEffect;
use super::model::Companion;

/// The name of the action that flips the mode, as a menu or the keymap's list says it.
pub const SWITCH_NAME: &str = "Switch Permissions Mode";

/// The name of the switch, as its tooltip says it before the key.
pub const TIP_NAME: &str = "Permissions mode";

/// The quiet note shown while a switch waits for the next step.
pub const FROM_NEXT_STEP: &str = "From the next step";

/// The help shown when an administrator holds every conversation in Manual.
pub const LOCKED_HELP: &str = "Your administrator keeps conversations in Manual.";

/// What the box says under the switch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ModeNote {
    /// Nothing.
    #[default]
    Quiet,
    /// A switch was made while a step was running; it holds from the next step, which starts when
    /// the answer says more.
    FromNextStep {
        /// How many progress lines the answer had when the switch was made.
        progress: usize,
    },
}

/// The conversation the input box types into.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConversationMode {
    /// The mode it is in.
    pub mode: Mode,
    /// True once the router has told the mode, or the person chose it: a turn carries it. False
    /// before anything opened: the box shows Auto as a placeholder and sends no mode with a turn.
    pub known: bool,
    /// The session the mode switches: the newest one the box opened.
    pub session: Option<SessionId>,
    /// True when an administrator holds every conversation in Manual: the switch does not work.
    pub locked: bool,
    /// What waits on the next step.
    pub note: ModeNote,
}

impl Default for ConversationMode {
    fn default() -> ConversationMode {
        ConversationMode {
            mode: Mode::Auto,
            known: false,
            session: None,
            locked: false,
            note: ModeNote::Quiet,
        }
    }
}

impl ConversationMode {
    /// The mode a turn from the box carries: none until the router has told one.
    pub fn turn_mode(&self) -> Option<Mode> {
        self.known.then_some(self.mode)
    }
}

/// What tells the conversation's mode something.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModeMsg {
    /// An ask opened `session`, which the router started in `mode`.
    Opened {
        /// The session.
        session: SessionId,
        /// The mode the router started it in.
        mode: Mode,
    },
    /// The router changed a session's mode, whoever asked.
    Changed {
        /// The session.
        session: SessionId,
        /// Its mode now.
        mode: Mode,
    },
    /// The person chose `mode`.
    Switch(Mode),
    /// The person flipped the mode (the key).
    Flip,
    /// The router refused a switch: an administrator holds every conversation in Manual.
    Locked,
    /// A switch to `mode` did not reach the router. The choice stands and rides the next turn.
    NotApplied(Mode),
}

/// What the box shows of the conversation's mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModeView {
    /// The selected side.
    pub mode: Mode,
    /// True when the switch does not work.
    pub locked: bool,
    /// The quiet note under it.
    pub note: Option<&'static str>,
    /// The help that says why the switch does not work.
    pub help: Option<&'static str>,
}

/// What the box shows of `conversation`.
pub fn mode_view(conversation: &ConversationMode) -> ModeView {
    ModeView {
        mode: conversation.mode,
        locked: conversation.locked,
        note: match conversation.note {
            ModeNote::Quiet => None,
            ModeNote::FromNextStep { .. } => Some(FROM_NEXT_STEP),
        },
        help: conversation.locked.then_some(LOCKED_HELP),
    }
}

/// The word for a mode on the switch.
pub fn mode_label(mode: Mode) -> &'static str {
    match mode {
        Mode::Auto => "Auto",
        Mode::Manual => "Manual",
    }
}

/// The other mode.
pub fn other_mode(mode: Mode) -> Mode {
    match mode {
        Mode::Auto => Mode::Manual,
        Mode::Manual => Mode::Auto,
    }
}

/// `state` after `msg`, and the switch it asks the router for.
pub fn update(state: Companion, msg: ModeMsg) -> (Companion, Vec<CompanionEffect>) {
    let current = state.mode.clone();
    match msg {
        ModeMsg::Opened { session, mode } => {
            let mode = match current.known {
                false => mode,
                true => current.mode,
            };
            let next = ConversationMode {
                mode,
                known: true,
                session: Some(session),
                ..current
            };
            (moded(state, next), Vec::new())
        }
        ModeMsg::Changed { session, mode } => match current.session.as_ref() == Some(&session) {
            true => {
                let next = ConversationMode {
                    mode,
                    known: true,
                    ..current
                };
                (moded(state, next), Vec::new())
            }
            false => (state, Vec::new()),
        },
        ModeMsg::Switch(mode) => switch(state, mode),
        ModeMsg::Flip => {
            let to = other_mode(current.mode);
            switch(state, to)
        }
        ModeMsg::Locked => {
            let next = ConversationMode {
                mode: Mode::Manual,
                known: true,
                locked: true,
                note: ModeNote::Quiet,
                ..current
            };
            (moded(state, next), Vec::new())
        }
        ModeMsg::NotApplied(mode) => {
            let next = ConversationMode {
                mode,
                known: true,
                note: ModeNote::Quiet,
                ..current
            };
            (moded(state, next), Vec::new())
        }
    }
}

/// The person chose `mode`: with a session the router is asked, and the box follows its answer;
/// without one the choice waits for the first turn.
fn switch(state: Companion, mode: Mode) -> (Companion, Vec<CompanionEffect>) {
    let current = state.mode.clone();
    if current.locked || current.mode == mode {
        return (state, Vec::new());
    }
    match current.session.clone() {
        None => {
            let next = ConversationMode {
                mode,
                known: true,
                ..current
            };
            (moded(state, next), Vec::new())
        }
        Some(session) => {
            let note = running_progress(&state).map_or(ModeNote::Quiet, |progress| {
                ModeNote::FromNextStep { progress }
            });
            let next = ConversationMode { note, ..current };
            (
                moded(state, next),
                vec![CompanionEffect::SetMode { session, mode }],
            )
        }
    }
}

/// A new turn starts a new step: the note is spent.
pub(super) fn turn_began(state: Companion) -> Companion {
    let next = ConversationMode {
        note: ModeNote::Quiet,
        ..state.mode.clone()
    };
    moded(state, next)
}

/// An answer moved: a note waiting for the next step is spent once the answer has taken one
/// (it said more) or is over.
pub(super) fn answer_moved(state: Companion) -> Companion {
    let ModeNote::FromNextStep { progress } = state.mode.note else {
        return state;
    };
    match running_progress(&state) {
        Some(now) if now <= progress => state,
        Some(_) | None => turn_began(state),
    }
}

/// How many progress lines the front answer has while it is still being made.
fn running_progress(state: &Companion) -> Option<usize> {
    let answer = state
        .front
        .as_ref()
        .and_then(|task| state.answers.get(task))?;
    match answer.wire.phase {
        AnswerPhase::Thinking | AnswerPhase::Streaming | AnswerPhase::NeedsYou(_) => {
            Some(answer.wire.progress.len())
        }
        AnswerPhase::Done | AnswerPhase::Failed | AnswerPhase::Cancelled => None,
    }
}

fn moded(state: Companion, mode: ConversationMode) -> Companion {
    Companion { mode, ..state }
}
