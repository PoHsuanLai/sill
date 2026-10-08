use companion_wire::{
    AnswerBody, AnswerPhase, AnswerWire, CardWire, FooterWire, FormWire, NeedsYou, PlanStepWire,
    PlanWire, RefusalWire, StepWireState,
};
use docket_core::{
    ActionRef, Args, BudgetKind, CallRefusal, CardActionId, ConfirmId, ContextKeep, DenyCode,
    EntityRef, Handle, Keep, LabelText, ParamDecl, ParamName, ParamNeed, ParamType, Reveal, StepId,
    TimeRange, UndoId, Value,
};
use ds::components::companion::answer::footer::ServedPlaceKind;
use ds::components::companion::answer::model::AnswerView;
use ds::components::companion::form::model::{FieldNeed, FormInput, FormValue};
use ds::components::companion::plan::model::{PlanFinish, PlanPhase, PlanStepState, StopWhy};
use ds_core::vocab::EffectMark;
use ds_core::word::Word;
use jiff::tz::TimeZone;
use porter_core::DataClass;
use prov::{AppName, EntityId, EntityKey, EntityKind, Label, Labelled, TaskId, UnixSeconds};
use sill_model::companion::RunState;

use super::*;

fn none(_: Handle) -> Option<String> {
    None
}

fn cx<'a>(zone: &'a TimeZone, display: &'a dyn Fn(Handle) -> Option<String>) -> ViewCx<'a> {
    ViewCx { zone, display }
}

fn app(name: &str) -> AppName {
    AppName::parse(name).unwrap_or_else(|e| panic!("{e}"))
}

fn label(text: &str) -> LabelText {
    LabelText::parse(text).unwrap_or_else(|e| panic!("{e}"))
}

fn person(name: &str, address: &str) -> EntityRef {
    EntityRef {
        id: EntityId {
            app: app("org.quire.Contacts"),
            kind: EntityKind::parse("contacts.person").unwrap_or_else(|e| panic!("{e}")),
            key: EntityKey::parse("p-1").unwrap_or_else(|e| panic!("{e}")),
        },
        title: Labelled::new(name.to_owned(), Label::trusted_user()),
        subtitle: Labelled::new(address.to_owned(), Label::trusted_user()),
    }
}

fn card(id: &str, effect: prov::Effect) -> CardWire {
    CardWire {
        id: CardActionId::parse(id).unwrap_or_else(|e| panic!("{e}")),
        label: label(id),
        effect,
        call: docket_core::CallRequest {
            action: ActionRef {
                app: app("org.quire.Mail"),
                name: prov::ActionName::parse("mail.message.send")
                    .unwrap_or_else(|e| panic!("{e}")),
            },
            target: docket_core::TargetValue::Nothing,
            args: Args::new(),
            origin: docket_core::Origin::Companion,
        },
    }
}

fn footer() -> FooterWire {
    FooterWire {
        served: Vec::new(),
        sources: vec![person("Sam Lee", "sam@example.org")],
        keep: ContextKeep {
            query: Keep::Kept,
            results: Keep::Dropped,
            selection: Keep::Dropped,
            window: Keep::Kept,
        },
    }
}

fn answer(phase: AnswerPhase, body: AnswerBody) -> AnswerWire {
    AnswerWire {
        task: TaskId::parse("t-1").unwrap_or_else(|e| panic!("{e}")),
        phase,
        body,
        footer: footer(),
    }
}

fn step(id: u32, state: StepWireState, effect: prov::Effect) -> PlanStepWire {
    PlanStepWire {
        id: StepId(id),
        action: ActionRef {
            app: app("org.quire.Mail"),
            name: prov::ActionName::parse("mail.message.forward").unwrap_or_else(|e| panic!("{e}")),
        },
        label: label("Forward"),
        effect,
        state,
        call: None,
    }
}

#[test]
fn effect_mark_total() {
    use prov::Effect as E;
    let table = [
        (E::Read, EffectMark::Read),
        (E::UndoableWrite, EffectMark::UndoableWrite),
        (E::Outbound, EffectMark::Outbound),
        (E::Destructive, EffectMark::Destructive),
        (E::Execute, EffectMark::Execute),
    ];
    for (effect, mark) in table {
        assert_eq!(effect_mark(effect), mark, "{effect:?}");
    }
}

/// Each state of sill's run is the view state of the same name (their words are `Word` slugs).
#[test]
fn run_state_total() {
    for state in RunState::ALL {
        assert_eq!(run_state(*state).slug(), state.slug(), "{state:?}");
    }
}

#[test]
fn run_place_total() {
    use docket_core::RunMode as M;
    for mode in [M::InPlace, M::AgentWorkspace, M::NestedSession] {
        let serde_name = serde_json::to_value(mode)
            .ok()
            .and_then(|v| v.as_str().map(str::to_owned))
            .unwrap_or_default();
        assert_eq!(
            run_place(mode).slug().replace('-', "_"),
            serde_name,
            "{mode:?}"
        );
    }
}

#[test]
fn answer_view_variants_total() {
    let zone = TimeZone::UTC;
    let display = |h: Handle| (h.0 == 7).then(|| "the receipts".to_owned());
    let cx = cx(&zone, &display);
    let reply = card("send", prov::Effect::Outbound);
    let range = TimeRange {
        from: UnixSeconds(1_790_000_000),
        to: UnixSeconds(1_790_003_600),
    };
    type Case = (&'static str, AnswerBody, &'static str);
    let cases: Vec<Case> = vec![
        (
            "text",
            AnswerBody::Text {
                lines: vec![Reveal::Plain("Two found.".to_owned())],
            },
            "Text",
        ),
        (
            "draft reply",
            AnswerBody::DraftReply {
                to: vec![person("Sam Lee", "sam@example.org")],
                subject: Reveal::Plain("Receipts".to_owned()),
                body: Reveal::Plain("Attached.".to_owned()),
                actions: vec![reply.clone()],
            },
            "DraftReply",
        ),
        (
            "proposed event",
            AnswerBody::ProposedEvent {
                title: "Lunch".to_owned(),
                when: range,
                people: Vec::new(),
                actions: vec![card("add", prov::Effect::UndoableWrite)],
            },
            "ProposedEvent",
        ),
        (
            "plan",
            AnswerBody::Plan(PlanWire {
                steps: vec![step(1, StepWireState::Pending, prov::Effect::Outbound)],
            }),
            "Plan",
        ),
        (
            "replace",
            AnswerBody::Replace {
                original: Reveal::Plain("teh".to_owned()),
                proposed: Reveal::Plain("the".to_owned()),
                undo: None,
            },
            "Replace",
        ),
        (
            "form",
            AnswerBody::Form(FormWire {
                action: ActionRef {
                    app: app("org.quire.Mail"),
                    name: prov::ActionName::parse("mail.draft.create")
                        .unwrap_or_else(|e| panic!("{e}")),
                },
                params: Vec::new(),
                values: Args::new(),
            }),
            "Form",
        ),
        (
            "refused",
            AnswerBody::Refused(RefusalWire::Failed("no".to_owned())),
            "Refused",
        ),
    ];
    for (name, body, want) in cases {
        let view = answer_view(&answer(AnswerPhase::Done, body), "ask", &cx);
        let got = match view {
            AnswerView::Text(_) => "Text",
            AnswerView::DraftReply(_) => "DraftReply",
            AnswerView::ProposedEvent(_) => "ProposedEvent",
            AnswerView::Plan(_) => "Plan",
            AnswerView::Replace(_) => "Replace",
            AnswerView::Form(_) => "Form",
            AnswerView::Refused(_) => "Refused",
        };
        assert_eq!(got, want, "{name}");
    }
}

#[test]
fn every_card_draws_its_footer() {
    let zone = TimeZone::UTC;
    let cx = cx(&zone, &none);
    let view = answer_view(
        &answer(AnswerPhase::Done, AnswerBody::Text { lines: Vec::new() }),
        "ask",
        &cx,
    );
    let AnswerView::Text(text) = view else {
        panic!("a text answer");
    };
    assert_eq!(
        text.footer.served_by.place.kind,
        ServedPlaceKind::ThisComputer
    );
    assert_eq!(text.footer.sources.len(), 1);
    assert_eq!(text.footer.sources[0].label, "Sam Lee");
    let kept: Vec<&str> = text.footer.scope.iter().map(|c| c.label.as_str()).collect();
    assert_eq!(
        kept,
        ["Search", "Window"],
        "the chips the person kept, and no others"
    );
}

#[test]
fn a_handle_reads_as_what_the_router_resolved_it_to_or_an_ellipsis() {
    let zone = TimeZone::UTC;
    let display = |h: Handle| (h.0 == 7).then(|| "the receipts".to_owned());
    let cx = cx(&zone, &display);
    let body = AnswerBody::Text {
        lines: vec![Reveal::Handle(Handle(7)), Reveal::Handle(Handle(8))],
    };
    let AnswerView::Text(text) = answer_view(&answer(AnswerPhase::Done, body), "ask", &cx) else {
        panic!("a text answer");
    };
    let words: Vec<String> = text.body.iter().map(|line| line.plain_text()).collect();
    assert_eq!(words, ["the receipts", "…"]);
}

#[test]
fn an_outbound_card_is_never_primary_without_its_mark_and_a_destructive_one_never_is() {
    let outbound = card_action(&card("send", prov::Effect::Outbound), true);
    assert_eq!(outbound.effect, EffectMark::Outbound);
    let destructive = card_action(&card("delete", prov::Effect::Destructive), true);
    assert_eq!(
        destructive.role,
        ds::components::companion::answer::action::ActionRole::Destructive,
        "even first in the list"
    );
    let second = card_action(&card("save", prov::Effect::UndoableWrite), false);
    assert_eq!(
        second.role,
        ds::components::companion::answer::action::ActionRole::Secondary
    );
}

#[test]
fn plan_phases_follow_the_answer_and_the_steps() {
    use prov::Effect as E;
    let zone = TimeZone::UTC;
    let cx = cx(&zone, &none);
    let done = || StepWireState::Done {
        undo: Some(UndoId(3)),
    };
    type Case = (&'static str, AnswerPhase, Vec<StepWireState>, PlanPhase);
    let confirm = AnswerPhase::NeedsYou(NeedsYou::Confirm(
        ConfirmId::parse("c-1").unwrap_or_else(|e| panic!("{e}")),
    ));
    let cases: Vec<Case> = vec![
        (
            "nothing run yet is a draft",
            AnswerPhase::Thinking,
            vec![StepWireState::Pending, StepWireState::Pending],
            PlanPhase::Draft,
        ),
        (
            "a running step",
            AnswerPhase::Streaming,
            vec![done(), StepWireState::Running],
            PlanPhase::Running {
                at: ds::components::companion::plan::model::StepId(2),
            },
        ),
        (
            "waiting on a confirmation stops at the step in hand",
            confirm,
            vec![done(), StepWireState::Running],
            PlanPhase::Stopped {
                at: ds::components::companion::plan::model::StepId(2),
                why: StopWhy::NeedsYou,
            },
        ),
        (
            "every step done",
            AnswerPhase::Done,
            vec![done(), done()],
            PlanPhase::Finished(PlanFinish::AllDone),
        ),
        (
            "a step left out",
            AnswerPhase::Done,
            vec![done(), StepWireState::Skipped],
            PlanPhase::Finished(PlanFinish::Partly),
        ),
        (
            "every step undone",
            AnswerPhase::Done,
            vec![StepWireState::Undone, StepWireState::Undone],
            PlanPhase::Undone,
        ),
        (
            "stopped by the person",
            AnswerPhase::Cancelled,
            vec![done(), StepWireState::Pending],
            PlanPhase::Stopped {
                at: ds::components::companion::plan::model::StepId(2),
                why: StopWhy::You,
            },
        ),
    ];
    for (name, phase, states, want) in cases {
        let steps = states
            .into_iter()
            .enumerate()
            .map(|(at, state)| step(u32::try_from(at + 1).unwrap_or(0), state, E::Outbound))
            .collect();
        let body = AnswerBody::Plan(PlanWire { steps });
        let AnswerView::Plan(view) = answer_view(&answer(phase, body), "forward", &cx) else {
            panic!("a plan");
        };
        assert_eq!(view.phase, want, "{name}");
    }
}

#[test]
fn plan_steps_group_by_effect_and_carry_why_they_failed() {
    use prov::Effect as E;
    let plan = PlanWire {
        steps: vec![
            step(1, StepWireState::Done { undo: None }, E::Read),
            step(2, StepWireState::Pending, E::Read),
            step(
                3,
                StepWireState::Failed(CallRefusal::OverBudget(BudgetKind::Outbound)),
                E::Outbound,
            ),
        ],
    };
    let zone = TimeZone::UTC;
    let cx = cx(&zone, &none);
    let AnswerView::Plan(view) = answer_view(
        &answer(AnswerPhase::Failed, AnswerBody::Plan(plan)),
        "forward",
        &cx,
    ) else {
        panic!("a plan");
    };
    let shape: Vec<(EffectMark, usize)> = view
        .groups
        .iter()
        .map(|g| (g.effect, g.steps.len()))
        .collect();
    assert_eq!(shape, [(EffectMark::Read, 2), (EffectMark::Outbound, 1)]);
    assert_eq!(
        view.groups[1].steps[0].state,
        PlanStepState::Failed("Over its budget (outbound actions).".to_owned())
    );
}

#[test]
fn refusal_words_total() {
    use docket_core::{ArgFault, ConfirmEnd};
    let refusals = vec![
        CallRefusal::App(docket_core::AppRefusal::NeedsParam {
            param: ParamName::parse("to").unwrap_or_else(|e| panic!("{e}")),
            options: Vec::new(),
        }),
        CallRefusal::Denied(DenyCode::OutsideTask),
        CallRefusal::Denied(DenyCode::NotAllowed),
        CallRefusal::Denied(DenyCode::NeedsUser),
        CallRefusal::Denied(DenyCode::Repeated),
        CallRefusal::Unconfirmed(ConfirmEnd::Refused),
        CallRefusal::Unconfirmed(ConfirmEnd::Dismissed),
        CallRefusal::Unconfirmed(ConfirmEnd::Expired),
        CallRefusal::Unconfirmed(ConfirmEnd::Cancelled),
        CallRefusal::OverBudget(BudgetKind::Spend),
        CallRefusal::BadArgs {
            param: ParamName::parse("to").unwrap_or_else(|e| panic!("{e}")),
            why: ArgFault::Missing,
        },
        CallRefusal::AppUnavailable(app("org.quire.Mail")),
        CallRefusal::Timeout,
    ];
    for refusal in refusals {
        assert!(!refusal_words(&refusal).is_empty(), "{refusal:?}");
    }
}

#[test]
fn a_form_field_carries_its_need_its_input_and_its_value() {
    let name = |n: &str| ParamName::parse(n).unwrap_or_else(|e| panic!("{e}"));
    let params = vec![
        ParamDecl {
            name: name("to"),
            label: label("To"),
            ty: ParamType::Entity(
                EntityKind::parse("contacts.person").unwrap_or_else(|e| panic!("{e}")),
            ),
            need: ParamNeed::Required,
            sink: docket_core::ArgSink::Recipient,
        },
        ParamDecl {
            name: name("note"),
            label: label("Note"),
            ty: ParamType::Text {
                max: docket_core::CharCount(200),
                lines: docket_core::Lines::Many,
            },
            need: ParamNeed::Optional,
            sink: docket_core::ArgSink::Body,
        },
    ];
    let mut values = Args::new();
    values.insert(
        name("note"),
        Labelled::new(Value::Text("hello".to_owned()), Label::trusted_user()),
    );
    let form = FormWire {
        action: ActionRef {
            app: app("org.quire.Mail"),
            name: prov::ActionName::parse("mail.draft.create").unwrap_or_else(|e| panic!("{e}")),
        },
        params,
        values,
    };
    let zone = TimeZone::UTC;
    let cx = cx(&zone, &none);
    let AnswerView::Form(view) = answer_view(
        &answer(AnswerPhase::Done, AnswerBody::Form(form)),
        "write to Sam",
        &cx,
    ) else {
        panic!("a form");
    };
    assert_eq!(view.title, "write to Sam");
    assert_eq!(view.fields[0].need, FieldNeed::Required);
    assert_eq!(view.fields[0].value, FormValue::Empty);
    assert!(matches!(view.fields[0].input, FormInput::Entity { .. }));
    assert_eq!(view.fields[1].need, FieldNeed::Optional);
    assert_eq!(view.fields[1].value, FormValue::Text("hello".to_owned()));
    assert_eq!(view.fields[1].input, FormInput::Lines);
}

#[test]
fn a_refusal_that_needs_the_cloud_offers_allow_once() {
    let zone = TimeZone::UTC;
    let cx = cx(&zone, &none);
    let body = AnswerBody::Refused(RefusalWire::NeedsCloud(DataClass::Mail));
    let AnswerView::Refused(refused) = answer_view(&answer(AnswerPhase::Failed, body), "ask", &cx)
    else {
        panic!("a refusal");
    };
    assert_eq!(refused.actions.len(), 1);
    assert_eq!(refused.actions[0].label, "Allow Once…");
}
