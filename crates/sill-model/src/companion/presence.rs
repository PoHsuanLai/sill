//! The companion's one presence, derived from what is happening (design/32 section 4.10).

use companion_wire::AnswerPhase;
use docket_core::RunMode;
use ds_core::vocab::CompanionPresence;

use super::model::Companion;
use super::run_state::{RunState, run_state_of};
use super::summon::Summon;

/// The presence of `state`, by priority: waiting on the person, then acting in a window, then
/// working, then taking a prompt, then idle.
pub fn presence_of(state: &Companion) -> CompanionPresence {
    let run_states = || {
        state
            .runs
            .values()
            .map(|run| (run_state_of(run.state, run.outcome), run.mode))
    };
    let waiting = state
        .answers
        .values()
        .any(|answer| matches!(answer.wire.phase, AnswerPhase::NeedsYou(_)))
        || run_states().any(|(run, _)| matches!(run, RunState::NeedsYou | RunState::TakenOver));
    let acting =
        run_states().any(|(run, place)| run == RunState::Running && place == RunMode::InPlace);
    let working = state.answers.values().any(|answer| {
        matches!(
            answer.wire.phase,
            AnswerPhase::Thinking | AnswerPhase::Streaming
        )
    }) || run_states()
        .any(|(run, _)| matches!(run, RunState::Starting | RunState::Running));
    let listening = matches!(
        state.summon,
        Summon::InApp { .. } | Summon::InLauncher { .. }
    );
    [
        (waiting, CompanionPresence::Waiting),
        (acting, CompanionPresence::Acting),
        (working, CompanionPresence::Working),
        (listening, CompanionPresence::Listening),
    ]
    .into_iter()
    .find_map(|(holds, presence)| holds.then_some(presence))
    .unwrap_or(CompanionPresence::Idle)
}
