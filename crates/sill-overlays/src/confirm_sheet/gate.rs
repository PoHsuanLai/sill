//! How the person may answer, pure and on quire's clock: the arming delay, Escape, and the hold a
//! destructive action asks for. `ds::machine::use_machine` runs it; the surface feeds it the
//! buttons' presses and carries out its one output.
//!
//! - **Arming.** The sheet takes answers only once it has been on screen for [`ARM_MS`] with no
//!   key pressed and the pointer still. Until then nothing answers but Escape: the buttons are
//!   disabled and a press, Return or a hold is ignored, and every key or pointer move on the
//!   sheet starts the wait again. A key or a click meant for the window behind, or a person still
//!   typing there, cannot allow anything; only someone who stopped and looked can.
//! - **Armed.** A press on a button answers with its choice; Escape denies.
//! - **Holding.** A destructive action's Allow takes a hold: it begins when the button goes down
//!   and answers `AllowOnce` once [`HOLD_MS`] have passed with it still down; letting go or
//!   leaving the button before then abandons it, back to armed.
//! - **Answered.** Done; nothing else counts (a second answer to one question is impossible).
//!
//! The delays are not settings yet (`confirm.arm_ms`, `confirm.hold_ms` are the proposed keys,
//! design/22 first); they are constants here with the proposed defaults.

use ds::base::machine::{Elapsed, Machine};
use ds::base::time::stamp::{Ms, Stamp};
use sill_model::intents::sheet::SheetChoice;

/// How long the buttons stay disabled after the sheet appears (`confirm.arm_ms`, proposed 500).
pub const ARM_MS: Ms = Ms(500);

/// How long a destructive action's Allow is held (`confirm.hold_ms`, proposed 1000).
pub const HOLD_MS: Ms = Ms(1000);

/// The gate's delays.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GateParams {
    pub arm_ms: Ms,
    pub hold_ms: Ms,
}

impl Default for GateParams {
    fn default() -> GateParams {
        GateParams {
            arm_ms: ARM_MS,
            hold_ms: HOLD_MS,
        }
    }
}

/// Where the person's answer stands. A gate starts [`AnswerGate::arming`]: the sheet's first
/// draw is its first state, so it has no state for a sheet not yet on screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnswerGate {
    /// On screen, not taking answers until `until`.
    Arming { until: Stamp },
    /// Taking answers.
    Armed,
    /// The Allow button of a destructive action is down since `since`, and answers at `until`.
    Holding { since: Stamp, until: Stamp },
    /// Answered; nothing else counts.
    Answered,
}

/// What moves the gate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GateIn {
    /// The clock: the time the gate asked to be woken at has come.
    Elapsed,
    /// A key went down or the pointer moved on the sheet: while arming, the wait starts again.
    Activity,
    /// A button was pressed.
    Press(SheetChoice),
    /// The Allow button of a destructive action went down.
    HoldBegan,
    /// It came up, or the pointer left it.
    HoldEnded,
    /// Escape.
    Escape,
}

impl From<Elapsed> for GateIn {
    fn from(_: Elapsed) -> GateIn {
        GateIn::Elapsed
    }
}

/// What the gate asks the surface to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GateOut {
    /// Send this answer.
    Answer(SheetChoice),
}

impl AnswerGate {
    /// The gate for a sheet that appears at `at`: arming for the arming delay.
    pub fn arming(at: Stamp, params: &GateParams) -> AnswerGate {
        AnswerGate::Arming {
            until: at.after(params.arm_ms),
        }
    }

    /// Whether the buttons take input.
    pub fn takes_input(&self) -> bool {
        matches!(self, AnswerGate::Armed | AnswerGate::Holding { .. })
    }
}

impl Machine for AnswerGate {
    type In = GateIn;
    type Out = GateOut;
    type Params = GateParams;
    type Ctx = ();

    fn step(
        self,
        input: GateIn,
        at: Stamp,
        params: &GateParams,
        _: &(),
    ) -> (AnswerGate, Vec<GateOut>) {
        let answer = |choice| (AnswerGate::Answered, vec![GateOut::Answer(choice)]);
        match (self, input) {
            (AnswerGate::Answered, _) => (self, Vec::new()),
            (_, GateIn::Escape) => answer(SheetChoice::Deny),
            (AnswerGate::Arming { until }, GateIn::Elapsed) if at >= until => {
                (AnswerGate::Armed, Vec::new())
            }
            (AnswerGate::Arming { .. }, GateIn::Activity) => (
                AnswerGate::Arming {
                    until: at.after(params.arm_ms),
                },
                Vec::new(),
            ),
            (AnswerGate::Armed, GateIn::Press(choice)) => answer(choice),
            (AnswerGate::Armed, GateIn::HoldBegan) => (
                AnswerGate::Holding {
                    since: at,
                    until: at.after(params.hold_ms),
                },
                Vec::new(),
            ),
            (AnswerGate::Holding { until, .. }, GateIn::Elapsed) if at >= until => {
                answer(SheetChoice::AllowOnce)
            }
            (AnswerGate::Holding { .. }, GateIn::HoldEnded) => (AnswerGate::Armed, Vec::new()),
            // A press while holding is the release's click; the hold decides, not the press.
            (state, _) => (state, Vec::new()),
        }
    }

    fn wake(&self) -> Option<Stamp> {
        match self {
            AnswerGate::Arming { until } | AnswerGate::Holding { until, .. } => Some(*until),
            AnswerGate::Armed | AnswerGate::Answered => None,
        }
    }
}
