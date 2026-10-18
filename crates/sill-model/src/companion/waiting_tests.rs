//! What assistants asked to remember: the rows' words, the empty list, and a row leaving once
//! the person answered.

use almanac_core::{
    Fact, FactId, FactState, FactText, FactView, SourceView, ThingView, TopicPath, UseCount,
    UserText, Validity,
};
use ds_core::time::stamp::Stamp;
use prov::{
    Actor, AgentLabel, AgentProgram, AppName, EntityId, EntityKey, EntityKind, Label, SystemPart,
};
use sill_launcher::Generation;

use super::{
    Companion, CompanionEvent, CompanionParams, LookupPhase, ProgramLabels, SETTLE_FAILED, step,
    waiting_line,
};
use super::helpers::HelpersModel;
use companion_wire::HelperLine;
use docket_core::helper::{HelperName, HelperPhase};
use prov::{AgentRole, HelperId, TaskId};

fn id(last: char) -> FactId {
    FactId::parse(&format!("{}{last}", "0".repeat(25))).unwrap_or_else(|e| panic!("{e}"))
}

fn view(last: char, text: &str, by: Actor, sources: Vec<SourceView>) -> FactView {
    FactView {
        fact: Fact {
            id: id(last),
            text: FactText::parse(text).unwrap_or_else(|e| panic!("{e}")),
            recorded: prov::UnixSeconds(1),
            by,
            label: Label::trusted_user(),
            links: Vec::new(),
            supersedes: Vec::new(),
            valid: Validity::Unstated,
        },
        topic: TopicPath::parse("prefs/meetings").unwrap_or_else(|e| panic!("{e}")),
        state: FactState::Pending,
        sources,
        used: UseCount(0),
        last_used: None,
        flagged: Vec::new(),
    }
}

fn from_notes() -> SourceView {
    SourceView::Present(ThingView {
        thing: EntityId {
            app: AppName::parse("org.quire.Notes").unwrap_or_else(|e| panic!("{e}")),
            kind: EntityKind::parse("notes.note").unwrap_or_else(|e| panic!("{e}")),
            key: EntityKey::parse("n1").unwrap_or_else(|e| panic!("{e}")),
        },
        title: UserText::new("Standup"),
        subtitle: UserText::new(""),
    })
}

fn agent() -> Actor {
    Actor::Acp {
        program: AgentProgram::parse("claude-code").unwrap_or_else(|e| panic!("{e}")),
        label: Some(AgentLabel("Claude Code".to_owned())),
    }
}

fn run(state: Companion, event: CompanionEvent) -> Companion {
    step(state, event, Stamp(0), &CompanionParams::default()).0
}

#[test]
fn a_row_says_the_words_and_who_asked_in_plain_words() {
    let programs = ProgramLabels::default();
    let cases = [
        (
            view('1', "Prefers mornings", Actor::Cli, Vec::new()),
            "A terminal asked to remember",
        ),
        (
            view('2', "Prefers mornings", agent(), Vec::new()),
            "Claude Code (assistant) asked to remember",
        ),
        (
            view(
                '4',
                "Prefers mornings",
                Actor::System {
                    part: SystemPart::Router,
                },
                Vec::new(),
            ),
            "The assistant asked to remember",
        ),
        (
            view(
                '5',
                "Prefers mornings",
                Actor::System {
                    part: SystemPart::Cua,
                },
                Vec::new(),
            ),
            "The system asked to remember",
        ),
        (
            view(
                '3',
                "Prefers mornings",
                Actor::Companion {
                    session: prov::SessionId::parse("s-1").unwrap_or_else(|e| panic!("{e}")),
                    role: prov::AgentRole::Planner,
                },
                vec![from_notes()],
            ),
            "Notes asked to remember",
        ),
    ];
    for (view, subtitle) in cases {
        let (title, said) = waiting_line(&view, &programs, &HelpersModel::default());
        assert_eq!(
            (title.as_str(), said.as_str()),
            ("Prefers mornings", subtitle)
        );
    }
}

fn helper_actor(helper: &str) -> Actor {
    Actor::Companion {
        session: prov::SessionId::parse("s-2").unwrap_or_else(|e| panic!("{e}")),
        role: AgentRole::Helper {
            helper: HelperId::parse(helper).unwrap_or_else(|e| panic!("{e}")),
            task: TaskId::parse("t-1").unwrap_or_else(|e| panic!("{e}")),
        },
    }
}

fn helpers_with(helper: &str, name: &str) -> HelpersModel {
    let mut helpers = HelpersModel::default();
    helpers.lines.push(HelperLine {
        id: HelperId::parse(helper).unwrap_or_else(|e| panic!("{e}")),
        name: HelperName::parse(name).unwrap_or_else(|e| panic!("{e}")),
        space: prov::SpaceId::desktop(),
        doing: String::new(),
        state: HelperPhase::Idle,
        needs_you: None,
        agent: None,
    });
    helpers
}

#[test]
fn a_helpers_row_names_the_helper_or_says_a_helper_when_the_list_lacks_it() {
    let programs = ProgramLabels::default();
    let proposal = view('6', "Prefers mornings", helper_actor("h1"), vec![from_notes()]);
    let cases = [
        (helpers_with("h1", "Finder 1"), "Finder 1 asked to remember"),
        (helpers_with("h2", "Finder 2"), "A helper asked to remember"),
        (HelpersModel::default(), "A helper asked to remember"),
    ];
    for (helpers, subtitle) in cases {
        let (_, said) = waiting_line(&proposal, &programs, &helpers);
        assert_eq!(said, subtitle);
    }
}

#[test]
fn nothing_waiting_is_an_empty_list() {
    let state = run(
        Companion::default(),
        CompanionEvent::WaitingLoaded(Generation(1), Vec::new()),
    );
    assert!(state.waiting.found.is_empty());
}

#[test]
fn an_answered_row_leaves_and_the_others_stay() {
    let (a, b) = (
        view('1', "One", Actor::Cli, Vec::new()),
        view('2', "Two", Actor::Cli, Vec::new()),
    );
    let state = run(
        Companion::default(),
        CompanionEvent::TimelineAsked(Generation(3)),
    );
    let state = run(
        state,
        CompanionEvent::WaitingLoaded(Generation(3), vec![a, b.clone()]),
    );
    assert_eq!(state.waiting.phase, LookupPhase::Answered);
    let state = run(state, CompanionEvent::Settled(id('1')));
    assert_eq!(state.waiting.found, vec![b]);
}

#[test]
fn a_failed_answer_is_a_sentence_until_the_next_one_goes_through() {
    let state = run(Companion::default(), CompanionEvent::SettleFailed);
    assert_eq!(state.memory_notice.as_deref(), Some(SETTLE_FAILED));
    let state = run(state, CompanionEvent::Settled(id('1')));
    assert_eq!(state.memory_notice, None);
}

#[test]
fn only_an_answer_to_a_waiting_memory_moves_the_count_the_filter_follows() {
    let g = Generation(4);
    let quiet = [
        CompanionEvent::TimelineAsked(g),
        CompanionEvent::WaitingLoaded(g, Vec::new()),
        CompanionEvent::WaitingFailed(g),
        CompanionEvent::TimelineLoaded(g, Vec::new()),
        CompanionEvent::TimelineFailed(g),
    ];
    let state = quiet.into_iter().fold(Companion::default(), run);
    assert_eq!(
        state.memory_answers, 0,
        "a timeline's own answer never asks again"
    );
    let state = run(state, CompanionEvent::Settled(id('1')));
    assert_eq!(state.memory_answers, 1);
    let state = run(state, CompanionEvent::SettleFailed);
    assert_eq!(state.memory_answers, 2, "the notice needs a new list too");
}
