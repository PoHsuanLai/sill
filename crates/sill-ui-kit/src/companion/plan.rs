//! A plan as the card draws it: the steps grouped by effect, and where the plan stands.

use companion_wire::{AnswerPhase, PlanStepWire, PlanWire, StepWireState};
use docket_core::StepId as WireStep;
use ds::components::companion::answer::footer::CardFooter;
use ds::components::companion::plan::model::{
    Inclusion, PlanFinish, PlanGroup, PlanPhase, PlanStep, PlanStepState, PlanView, StepId, StopWhy,
};
use ds_core::vocab::EffectMark;

use super::marks::effect_mark;
use super::words::refusal_words;

/// The plan, titled `title` (what the person asked), under `phase` (the answer's own).
pub fn plan_view(
    plan: &PlanWire,
    phase: &AnswerPhase,
    title: &str,
    footer: CardFooter,
) -> PlanView {
    let steps: Vec<PlanStep> = plan.steps.iter().map(step).collect();
    PlanView {
        title: title.to_owned(),
        groups: groups(steps),
        phase: plan_phase(plan, phase),
        footer,
    }
}

fn step(step: &PlanStepWire) -> PlanStep {
    PlanStep {
        id: StepId(step_number(step.id)),
        label: step.label.as_str().to_owned(),
        detail: None,
        app: None,
        effect: effect_mark(step.effect),
        inclusion: match step.state {
            StepWireState::Skipped => Inclusion::Excluded,
            StepWireState::Pending
            | StepWireState::Running
            | StepWireState::Done { .. }
            | StepWireState::Failed(_)
            | StepWireState::Undone => Inclusion::Included,
        },
        state: match &step.state {
            StepWireState::Pending => PlanStepState::Pending,
            StepWireState::Running => PlanStepState::Running,
            StepWireState::Done { .. } => PlanStepState::Done,
            StepWireState::Failed(refusal) => PlanStepState::Failed(refusal_words(refusal)),
            StepWireState::Skipped => PlanStepState::Skipped,
            StepWireState::Undone => PlanStepState::Undone,
        },
    }
}

fn step_number(id: WireStep) -> u32 {
    id.0
}

/// Consecutive steps of one effect share a group, in the order the plan runs.
fn groups(steps: Vec<PlanStep>) -> Vec<PlanGroup> {
    steps
        .into_iter()
        .fold(Vec::new(), |mut groups: Vec<PlanGroup>, step| {
            match groups.last_mut() {
                Some(last) if last.effect == step.effect => last.steps.push(step),
                _ => groups.push(PlanGroup {
                    label: effect_words(step.effect).to_owned(),
                    effect: step.effect,
                    steps: vec![step],
                }),
            }
            groups
        })
}

fn effect_words(effect: EffectMark) -> &'static str {
    match effect {
        EffectMark::Read => "Look up",
        EffectMark::UndoableWrite => "Change",
        EffectMark::Outbound => "Send",
        EffectMark::Execute => "Run",
        EffectMark::Destructive => "Delete",
    }
}

/// Where the plan stands: the answer's phase says whether it is over or needs the person, the
/// steps say where it is.
fn plan_phase(plan: &PlanWire, phase: &AnswerPhase) -> PlanPhase {
    let at = |wanted: fn(&StepWireState) -> bool| {
        plan.steps
            .iter()
            .find(|step| wanted(&step.state))
            .map(|step| StepId(step_number(step.id)))
    };
    let first = || {
        plan.steps
            .first()
            .map_or(StepId(0), |step| StepId(step_number(step.id)))
    };
    let all_undone = !plan.steps.is_empty()
        && plan
            .steps
            .iter()
            .all(|step| matches!(step.state, StepWireState::Undone | StepWireState::Skipped));
    match phase {
        AnswerPhase::Thinking | AnswerPhase::Streaming => {
            match at(|s| *s == StepWireState::Running) {
                Some(running) => PlanPhase::Running { at: running },
                None if plan.steps.iter().all(|s| s.state == StepWireState::Pending) => {
                    PlanPhase::Draft
                }
                None => PlanPhase::Running {
                    at: at(|s| *s == StepWireState::Pending).unwrap_or_else(first),
                },
            }
        }
        AnswerPhase::NeedsYou(_) => PlanPhase::Stopped {
            at: at(|s| matches!(s, StepWireState::Running | StepWireState::Pending))
                .unwrap_or_else(first),
            why: StopWhy::NeedsYou,
        },
        AnswerPhase::Failed => PlanPhase::Stopped {
            at: at(|s| matches!(s, StepWireState::Failed(_))).unwrap_or_else(first),
            why: StopWhy::Failed,
        },
        AnswerPhase::Cancelled => PlanPhase::Stopped {
            at: at(|s| matches!(s, StepWireState::Pending | StepWireState::Running))
                .unwrap_or_else(first),
            why: StopWhy::You,
        },
        AnswerPhase::Done if all_undone => PlanPhase::Undone,
        AnswerPhase::Done => {
            if plan
                .steps
                .iter()
                .all(|s| matches!(s.state, StepWireState::Done { .. }))
            {
                PlanPhase::Finished(PlanFinish::AllDone)
            } else {
                PlanPhase::Finished(PlanFinish::Partly)
            }
        }
    }
}
