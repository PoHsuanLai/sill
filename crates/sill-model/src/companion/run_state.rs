//! What a person sees of a run's state, derived from cuad's phase and outcome, and the controls
//! each state offers. Both are total tables.

use cua_bus::{CuaOutcome, RunStateSlug};
use docket_core::RunMode;
use ds_core::word::Word;
use prov::RunId;
use sill_launcher::RunControl;

use super::model::Companion;

/// A run as a row words it. The view enum of quire's run row has the same variants; the
/// conversion between them is `sill-ui-kit`'s `*_total` test.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Word)]
pub enum RunState {
    /// Asking consent or leasing a window.
    Starting,
    /// Looking, thinking or acting.
    Running,
    /// Paused.
    Paused,
    /// Waiting for the person: a question or a confirmation.
    NeedsYou,
    /// The person has the window.
    TakenOver,
    /// Finished what it was asked.
    Done,
    /// Could not.
    Failed,
    /// Stopped by someone.
    Stopped,
    /// Ran out of a budget.
    BudgetOut,
}

impl RunState {
    /// Whether the run still holds a window or a question: not over.
    pub fn is_live(self) -> bool {
        match self {
            RunState::Starting
            | RunState::Running
            | RunState::Paused
            | RunState::NeedsYou
            | RunState::TakenOver => true,
            RunState::Done | RunState::Failed | RunState::Stopped | RunState::BudgetOut => false,
        }
    }
}

/// The state a run is in, from cuad's phase and, for a finished run, how it ended. A finished
/// run whose outcome has not arrived yet reads `Stopped`: it is over, and nothing says it
/// succeeded.
pub fn run_state_of(slug: RunStateSlug, outcome: Option<CuaOutcome>) -> RunState {
    match slug {
        RunStateSlug::Granting | RunStateSlug::Leasing => RunState::Starting,
        RunStateSlug::Observing
        | RunStateSlug::Thinking
        | RunStateSlug::Authorising
        | RunStateSlug::Acting
        | RunStateSlug::Settling
        | RunStateSlug::Verifying => RunState::Running,
        RunStateSlug::Confirming | RunStateSlug::WaitingForUser => RunState::NeedsYou,
        RunStateSlug::Paused => RunState::Paused,
        RunStateSlug::TakenOver => RunState::TakenOver,
        RunStateSlug::Finished => match outcome {
            Some(CuaOutcome::Done) => RunState::Done,
            Some(CuaOutcome::Failed(_) | CuaOutcome::Blocked(_)) => RunState::Failed,
            Some(CuaOutcome::Cancelled(_)) | None => RunState::Stopped,
            Some(CuaOutcome::BudgetOut(_)) => RunState::BudgetOut,
            // An outcome this build does not know: nothing says it succeeded.
            Some(_) => RunState::Stopped,
        },
    }
}

/// The controls a run's row offers, in the order they are drawn. A window the person cannot see
/// (an agent workspace or a nested session) can be watched; an ended run offers to undo what it
/// did, when something it did can be undone.
pub fn run_controls(state: RunState, place: RunMode, undoable: u32) -> Vec<RunControl> {
    use RunControl as C;
    let hidden = match place {
        RunMode::InPlace => None,
        RunMode::AgentWorkspace | RunMode::NestedSession => Some(C::Watch),
    };
    // A finished run has nothing left to watch; one that ended badly leaves its last frame.
    let watch = match state {
        RunState::Done => None,
        _ => hidden,
    };
    let rest: &[RunControl] = match state {
        RunState::Starting | RunState::Running => &[C::Pause, C::TakeOver, C::Stop],
        RunState::Paused => &[C::Resume, C::TakeOver, C::Stop],
        RunState::NeedsYou => &[C::Stop],
        RunState::TakenOver => return vec![C::HandBack, C::Stop],
        RunState::Done => &[],
        RunState::Failed | RunState::Stopped | RunState::BudgetOut => &[],
    };
    let undo = (!state.is_live() && undoable > 0).then_some(C::UndoAll);
    watch
        .into_iter()
        .chain(rest.iter().copied())
        .chain(undo)
        .collect()
}

/// The runs that are going now (not paused, not over), which "stop everything" pauses. A run
/// waiting on the person, or already taken over, is going too: pausing it holds it where it is.
pub fn going_runs(companion: &Companion) -> Vec<RunId> {
    companion
        .runs
        .values()
        .filter(|run| {
            run_state_of(run.state, run.outcome).is_live() && run.state != RunStateSlug::Paused
        })
        .map(|run| run.run.clone())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use cua_bus::{BlockedOn, CancelledBy, FailReason, RunBudgetKind};

    #[test]
    fn every_phase_reads_as_one_state() {
        use RunStateSlug as S;
        let table: &[(S, Option<CuaOutcome>, RunState)] = &[
            (S::Granting, None, RunState::Starting),
            (S::Leasing, None, RunState::Starting),
            (S::Observing, None, RunState::Running),
            (S::Thinking, None, RunState::Running),
            (S::Authorising, None, RunState::Running),
            (S::Acting, None, RunState::Running),
            (S::Settling, None, RunState::Running),
            (S::Verifying, None, RunState::Running),
            (S::Confirming, None, RunState::NeedsYou),
            (S::WaitingForUser, None, RunState::NeedsYou),
            (S::Paused, None, RunState::Paused),
            (S::TakenOver, None, RunState::TakenOver),
            (S::Finished, Some(CuaOutcome::Done), RunState::Done),
            (
                S::Finished,
                Some(CuaOutcome::Failed(FailReason::WindowGone)),
                RunState::Failed,
            ),
            (
                S::Finished,
                Some(CuaOutcome::Blocked(BlockedOn::ConsentDenied)),
                RunState::Failed,
            ),
            (
                S::Finished,
                Some(CuaOutcome::Cancelled(CancelledBy::User)),
                RunState::Stopped,
            ),
            (
                S::Finished,
                Some(CuaOutcome::Cancelled(CancelledBy::KillSwitch)),
                RunState::Stopped,
            ),
            (
                S::Finished,
                Some(CuaOutcome::BudgetOut(RunBudgetKind::Spend)),
                RunState::BudgetOut,
            ),
            (S::Finished, None, RunState::Stopped),
        ];
        for (slug, outcome, want) in table {
            assert_eq!(run_state_of(*slug, *outcome), *want, "{slug:?} {outcome:?}");
        }
    }

    /// Design/32 section 4.6, with the replay row dropped (procedures come later) and "undo all"
    /// shown only when the run did something that can be undone.
    #[test]
    fn run_controls_table() {
        use RunControl as C;
        use RunMode as P;
        use RunState as S;
        type Case = (S, P, u32, &'static [RunControl]);
        #[rustfmt::skip]
        let table: &[Case] = &[
            (S::Starting,  P::InPlace,        0, &[C::Pause, C::TakeOver, C::Stop]),
            (S::Running,   P::InPlace,        3, &[C::Pause, C::TakeOver, C::Stop]),
            (S::Running,   P::AgentWorkspace, 0, &[C::Watch, C::Pause, C::TakeOver, C::Stop]),
            (S::Running,   P::NestedSession,  0, &[C::Watch, C::Pause, C::TakeOver, C::Stop]),
            (S::Paused,    P::InPlace,        0, &[C::Resume, C::TakeOver, C::Stop]),
            (S::Paused,    P::AgentWorkspace, 0, &[C::Watch, C::Resume, C::TakeOver, C::Stop]),
            (S::NeedsYou,  P::InPlace,        0, &[C::Stop]),
            (S::NeedsYou,  P::AgentWorkspace, 0, &[C::Watch, C::Stop]),
            (S::TakenOver, P::InPlace,        0, &[C::HandBack, C::Stop]),
            (S::TakenOver, P::AgentWorkspace, 0, &[C::HandBack, C::Stop]),
            (S::Done,      P::InPlace,        2, &[C::UndoAll]),
            (S::Done,      P::InPlace,        0, &[]),
            (S::Done,      P::AgentWorkspace, 2, &[C::UndoAll]),
            (S::Failed,    P::InPlace,        1, &[C::UndoAll]),
            (S::Stopped,   P::AgentWorkspace, 1, &[C::Watch, C::UndoAll]),
            (S::BudgetOut, P::NestedSession,  0, &[C::Watch]),
        ];
        for (state, place, undoable, want) in table {
            assert_eq!(
                run_controls(*state, *place, *undoable),
                *want,
                "{state:?} {place:?} undoable {undoable}"
            );
        }
    }
}
