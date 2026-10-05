//! The small marks: an effect, a run's state and where it acts.

use docket_core::RunMode;
use ds::components::companion::run_row::model::{RunPlace, RunState as RunStateView};
use ds_core::vocab::EffectMark;
use prov::Effect;
use sill_model::companion::RunState;

/// How much a call can change, for drawing.
pub fn effect_mark(effect: Effect) -> EffectMark {
    match effect {
        Effect::Read => EffectMark::Read,
        Effect::UndoableWrite => EffectMark::UndoableWrite,
        Effect::Outbound => EffectMark::Outbound,
        Effect::Destructive => EffectMark::Destructive,
    }
}

/// A run's state, for its row.
pub fn run_state(state: RunState) -> RunStateView {
    match state {
        RunState::Starting => RunStateView::Starting,
        RunState::Running => RunStateView::Running,
        RunState::Paused => RunStateView::Paused,
        RunState::NeedsYou => RunStateView::NeedsYou,
        RunState::TakenOver => RunStateView::TakenOver,
        RunState::Done => RunStateView::Done,
        RunState::Failed => RunStateView::Failed,
        RunState::Stopped => RunStateView::Stopped,
        RunState::BudgetOut => RunStateView::BudgetOut,
    }
}

/// Where a run acts, for its row.
pub fn run_place(mode: RunMode) -> RunPlace {
    match mode {
        RunMode::InPlace => RunPlace::InPlace,
        RunMode::AgentWorkspace => RunPlace::AgentWorkspace,
        RunMode::NestedSession => RunPlace::NestedSession,
    }
}
