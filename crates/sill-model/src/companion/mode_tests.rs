//! The conversation's mode: what opens it, what it follows, what a switch asks of the router,
//! the lock, the note while a step runs, and the mode a turn carries.

use companion_wire::{AnswerBody, AnswerPhase, AnswerWire, FooterWire};
use docket_core::{ContextKeep, Keep};
use docket_permissions_core::Mode;
use ds_core::time::stamp::Stamp;
use prov::{SessionId, TaskId};

use super::{
    Companion, CompanionEffect, CompanionEvent, CompanionParams, FROM_NEXT_STEP, Knowing,
    LOCKED_HELP, ModeLock, ModeMsg, mode_view, step,
};

fn session(id: &str) -> SessionId {
    SessionId::parse(id).unwrap_or_else(|e| panic!("{e}"))
}

fn opened(id: &str, mode: Mode) -> CompanionEvent {
    CompanionEvent::Mode(ModeMsg::Opened {
        session: session(id),
        mode,
    })
}

fn changed(id: &str, mode: Mode) -> CompanionEvent {
    CompanionEvent::Mode(ModeMsg::Changed {
        session: session(id),
        mode,
    })
}

fn switch(mode: Mode) -> CompanionEvent {
    CompanionEvent::Mode(ModeMsg::Switch(mode))
}

fn asked() -> CompanionEvent {
    CompanionEvent::Asked {
        task: TaskId::parse("t-1").unwrap_or_else(|e| panic!("{e}")),
        prompt: "tidy the inbox".to_owned(),
        session: session("s-1"),
    }
}

/// The answer to `asked`, in `phase`, having said `lines` on its way.
fn answer(phase: AnswerPhase, lines: usize) -> CompanionEvent {
    CompanionEvent::Answer(Box::new(AnswerWire {
        task: TaskId::parse("t-1").unwrap_or_else(|e| panic!("{e}")),
        phase,
        body: AnswerBody::Text { lines: Vec::new() },
        progress: (0..lines).map(|n| format!("step {n}")).collect(),
        footer: FooterWire {
            mode: docket_permissions_core::Mode::default(),
            served: Vec::new(),
            sources: Vec::new(),
            keep: ContextKeep {
                query: Keep::Dropped,
                results: Keep::Dropped,
                selection: Keep::Dropped,
                window: Keep::Dropped,
            },
        },
    }))
}

/// What the box shows and carries after the events, and the effects the last one asked for.
struct Seen {
    mode: Mode,
    knowing: Knowing,
    lock: ModeLock,
    note: Option<&'static str>,
    help: Option<&'static str>,
    turn: Option<Mode>,
    effects: Vec<CompanionEffect>,
}

fn run(events: Vec<CompanionEvent>) -> Seen {
    let mut state = Companion::default();
    let mut effects = Vec::new();
    for event in events {
        (state, effects) = step(state, event, Stamp(0), &CompanionParams::default());
    }
    let view = mode_view(&state.mode);
    Seen {
        mode: view.mode,
        knowing: state.mode.knowing,
        lock: view.lock,
        note: view.note,
        help: view.help,
        turn: state.mode.turn_mode(),
        effects,
    }
}

fn set_mode(id: &str, mode: Mode) -> Vec<CompanionEffect> {
    vec![CompanionEffect::SetMode {
        session: session(id),
        mode,
    }]
}

/// One scripted conversation and what the box shows at its end.
struct Case {
    name: &'static str,
    events: Vec<CompanionEvent>,
    mode: Mode,
    knowing: Knowing,
    turn: Option<Mode>,
    effects: Vec<CompanionEffect>,
    note: Option<&'static str>,
}

#[test]
fn the_box_follows_the_router_and_asks_it_for_a_switch() {
    let flip = CompanionEvent::Mode(ModeMsg::Flip);
    let cases = vec![
        Case {
            name: "nothing opened: Auto as a placeholder and no mode on a turn",
            events: vec![],
            mode: Mode::Auto,
            knowing: Knowing::Assumed,
            turn: None,
            effects: vec![],
            note: None,
        },
        Case {
            name: "a session opened in Manual shows Manual and carries it",
            events: vec![opened("s-1", Mode::Manual)],
            mode: Mode::Manual,
            knowing: Knowing::Known,
            turn: Some(Mode::Manual),
            effects: vec![],
            note: None,
        },
        Case {
            name: "a later session keeps the conversation's mode, not Settings'",
            events: vec![
                opened("s-1", Mode::Auto),
                switch(Mode::Manual),
                changed("s-1", Mode::Manual),
                opened("s-2", Mode::Auto),
            ],
            mode: Mode::Manual,
            knowing: Knowing::Known,
            turn: Some(Mode::Manual),
            effects: vec![],
            note: None,
        },
        Case {
            name: "ModeChanged on the session moves the box",
            events: vec![opened("s-1", Mode::Auto), changed("s-1", Mode::Manual)],
            mode: Mode::Manual,
            knowing: Knowing::Known,
            turn: Some(Mode::Manual),
            effects: vec![],
            note: None,
        },
        Case {
            name: "ModeChanged on another session is not this conversation's",
            events: vec![opened("s-1", Mode::Auto), changed("s-9", Mode::Manual)],
            mode: Mode::Auto,
            knowing: Knowing::Known,
            turn: Some(Mode::Auto),
            effects: vec![],
            note: None,
        },
        Case {
            name: "a switch asks the router and waits for its word",
            events: vec![opened("s-1", Mode::Auto), switch(Mode::Manual)],
            mode: Mode::Auto,
            knowing: Knowing::Known,
            turn: Some(Mode::Auto),
            effects: set_mode("s-1", Mode::Manual),
            note: None,
        },
        Case {
            name: "the key flips to the other mode",
            events: vec![opened("s-1", Mode::Manual), flip],
            mode: Mode::Manual,
            knowing: Knowing::Known,
            turn: Some(Mode::Manual),
            effects: set_mode("s-1", Mode::Auto),
            note: None,
        },
        Case {
            name: "a switch to the mode it is in asks nothing",
            events: vec![opened("s-1", Mode::Auto), switch(Mode::Auto)],
            mode: Mode::Auto,
            knowing: Knowing::Known,
            turn: Some(Mode::Auto),
            effects: vec![],
            note: None,
        },
        Case {
            name: "a switch before any session waits for the first turn",
            events: vec![switch(Mode::Manual)],
            mode: Mode::Manual,
            knowing: Knowing::Known,
            turn: Some(Mode::Manual),
            effects: vec![],
            note: None,
        },
        Case {
            name: "a switch that did not reach the router stands and rides the next turn",
            events: vec![
                opened("s-1", Mode::Auto),
                switch(Mode::Manual),
                CompanionEvent::Mode(ModeMsg::NotApplied(Mode::Manual)),
            ],
            mode: Mode::Manual,
            knowing: Knowing::Known,
            turn: Some(Mode::Manual),
            effects: vec![],
            note: None,
        },
        Case {
            name: "a switch while a step runs says it holds from the next step",
            events: vec![
                opened("s-1", Mode::Auto),
                asked(),
                answer(AnswerPhase::Thinking, 1),
                switch(Mode::Manual),
            ],
            mode: Mode::Auto,
            knowing: Knowing::Known,
            turn: Some(Mode::Auto),
            effects: set_mode("s-1", Mode::Manual),
            note: Some(FROM_NEXT_STEP),
        },
        Case {
            name: "the note stays while the same step goes on",
            events: vec![
                opened("s-1", Mode::Auto),
                asked(),
                answer(AnswerPhase::Thinking, 1),
                switch(Mode::Manual),
                changed("s-1", Mode::Manual),
                answer(AnswerPhase::Streaming, 1),
            ],
            mode: Mode::Manual,
            knowing: Knowing::Known,
            turn: Some(Mode::Manual),
            effects: vec![],
            note: Some(FROM_NEXT_STEP),
        },
        Case {
            name: "the note goes when the next step starts",
            events: vec![
                opened("s-1", Mode::Auto),
                asked(),
                answer(AnswerPhase::Thinking, 1),
                switch(Mode::Manual),
                changed("s-1", Mode::Manual),
                answer(AnswerPhase::Thinking, 2),
            ],
            mode: Mode::Manual,
            knowing: Knowing::Known,
            turn: Some(Mode::Manual),
            effects: vec![],
            note: None,
        },
        Case {
            name: "the note goes when the answer is over",
            events: vec![
                opened("s-1", Mode::Auto),
                asked(),
                answer(AnswerPhase::Thinking, 1),
                switch(Mode::Manual),
                answer(AnswerPhase::Done, 1),
            ],
            mode: Mode::Auto,
            knowing: Knowing::Known,
            turn: Some(Mode::Auto),
            effects: vec![],
            note: None,
        },
        Case {
            name: "a switch with no step running has nothing to wait for",
            events: vec![
                opened("s-1", Mode::Auto),
                asked(),
                answer(AnswerPhase::Done, 1),
                switch(Mode::Manual),
            ],
            mode: Mode::Auto,
            knowing: Knowing::Known,
            turn: Some(Mode::Auto),
            effects: set_mode("s-1", Mode::Manual),
            note: None,
        },
    ];
    for case in cases {
        let seen = run(case.events);
        assert_eq!(seen.mode, case.mode, "{}: mode", case.name);
        assert_eq!(seen.knowing, case.knowing, "{}: knowing", case.name);
        assert_eq!(seen.turn, case.turn, "{}: the turn's mode", case.name);
        assert_eq!(seen.effects, case.effects, "{}: effects", case.name);
        assert_eq!(seen.note, case.note, "{}: note", case.name);
    }
}

#[test]
fn a_locked_conversation_shows_manual_and_cannot_switch() {
    let locked = vec![
        opened("s-1", Mode::Auto),
        switch(Mode::Manual),
        CompanionEvent::Mode(ModeMsg::Locked),
        switch(Mode::Auto),
    ];
    let seen = run(locked);
    assert_eq!(seen.mode, Mode::Manual);
    assert_eq!(seen.lock, ModeLock::Locked);
    assert_eq!(seen.help, Some(LOCKED_HELP));
    assert_eq!(seen.note, None, "the refused switch leaves no note");
    assert_eq!(seen.effects, Vec::new(), "a locked switch asks nothing");
    let flipped = run(vec![
        opened("s-1", Mode::Manual),
        CompanionEvent::Mode(ModeMsg::Locked),
        CompanionEvent::Mode(ModeMsg::Flip),
    ]);
    assert_eq!(
        flipped.effects,
        Vec::new(),
        "the key is as locked as the control"
    );
    assert_eq!(run(vec![opened("s-1", Mode::Auto)]).help, None);
}
