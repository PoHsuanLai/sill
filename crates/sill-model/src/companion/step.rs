//! The companion service's one transition: the machines that decide where a summon goes, and the
//! bookkeeping of answers, runs, the journal and the two lookups.

use std::collections::BTreeMap;

use companion_wire::{AnswerBody, AnswerPhase, AnswerWire, FooterWire};
use docket_core::{ContextKeep, Hit, Keep};
use ds_core::machine::{Elapsed, Machine};
use ds_core::time::stamp::Stamp;
use prov::TaskId;
use sill_launcher::Generation;

use super::card::{card_call, handle_fallback, settled};
use super::command::{CompanionEffect, CompanionEvent, CompanionParams};
use super::helpers::{HelpersMsg, update};
use super::mode::{answer_moved, turn_began, update as update_mode};
use super::model::{AnswerFacts, Busy, Companion, Lookup, LookupPhase};
use super::model_picker::{Picker, opens, picker_of, touched};
use super::summon::{SummonCx, SummonIn, SummonOut};
use super::tap::{TapIn, TapOut};
use super::terminals::{TerminalsMsg, update as update_terminals};
use super::words::SETTLE_FAILED;

/// `state` after `event` at `at`, and what the outside must do.
pub fn step(
    state: Companion,
    event: CompanionEvent,
    at: Stamp,
    params: &CompanionParams,
) -> (Companion, Vec<CompanionEffect>) {
    match event {
        CompanionEvent::Link(link) => (Companion { link, ..state }, Vec::new()),
        CompanionEvent::Tapped(facts) => {
            let (tap, taps) = state.tap.step(TapIn::Tap, at, &params.tap, &());
            let state = Companion { tap, ..state };
            match taps.as_slice() {
                [TapOut::Summon] => summoned(state, SummonIn::Summon(facts), at, params),
                _ => (state, Vec::new()),
            }
        }
        CompanionEvent::Summoned(facts) => summoned(state, SummonIn::Summon(facts), at, params),
        CompanionEvent::AppAnswered(serial, answer) => {
            summoned(state, SummonIn::AppAnswered(serial, answer), at, params)
        }
        CompanionEvent::Elapsed => {
            let (tap, _) = state.tap.step(TapIn::from(Elapsed), at, &params.tap, &());
            summoned(
                Companion { tap, ..state },
                SummonIn::from(Elapsed),
                at,
                params,
            )
        }
        CompanionEvent::LauncherClosed => {
            let state = Companion {
                memory_notice: None,
                ..state
            };
            summoned(state, SummonIn::LauncherClosed, at, params)
        }
        CompanionEvent::Dismissed => summoned(state, SummonIn::Dismiss, at, params),
        CompanionEvent::Helpers(msg) => helped(state, *msg),
        CompanionEvent::Terminals(msg) => in_terminals(state, msg),
        CompanionEvent::Mode(msg) => update_mode(state, msg),
        other => {
            let task = touched(&other).cloned();
            let before = task
                .as_ref()
                .map_or(Picker::Unneeded, |task| picker_of(&state, task));
            let state = bookkeeping(state, other);
            let after = task
                .as_ref()
                .map_or(Picker::Unneeded, |task| picker_of(&state, task));
            (state, opens(before, after))
        }
    }
}

/// The sessions in terminals move, and what they ask of the companion becomes effects.
fn in_terminals(state: Companion, msg: TerminalsMsg) -> (Companion, Vec<CompanionEffect>) {
    let (terminals, commands) = update_terminals(state.terminals, msg);
    (
        Companion { terminals, ..state },
        commands
            .into_iter()
            .map(CompanionEffect::Terminals)
            .collect(),
    )
}

/// The helpers screen moves, and what it asks of the companion becomes effects.
fn helped(state: Companion, msg: HelpersMsg) -> (Companion, Vec<CompanionEffect>) {
    let (helpers, commands) = update(state.helpers, msg);
    (
        Companion { helpers, ..state },
        commands.into_iter().map(CompanionEffect::Helpers).collect(),
    )
}

/// When the machines next want to be stepped with no input: the earlier of their wakes.
pub fn wake(state: &Companion) -> Option<Stamp> {
    [state.tap.wake(), state.summon.wake()]
        .into_iter()
        .flatten()
        .min()
}

/// Step the summon machine, remember the serial it used, and turn its outputs into effects. A
/// summon that opens a prompt warms the planner first.
fn summoned(
    state: Companion,
    input: SummonIn,
    at: Stamp,
    params: &CompanionParams,
) -> (Companion, Vec<CompanionEffect>) {
    let cx = SummonCx {
        next: docket_core::SummonSerial(state.asked_serial + 1),
    };
    let (summon, outs) = state.summon.clone().step(input, at, &params.summon, &cx);
    let asked_serial = outs
        .iter()
        .find_map(|out| match out {
            SummonOut::AskApp { serial, .. } => Some(serial.0),
            _ => None,
        })
        .unwrap_or(state.asked_serial);
    let warm = outs
        .iter()
        .any(SummonOut::opens_prompt)
        .then_some(CompanionEffect::Prepare);
    let effects = warm
        .into_iter()
        .chain(outs.into_iter().map(CompanionEffect::Summon))
        .collect();
    (
        Companion {
            summon,
            asked_serial,
            ..state
        },
        effects,
    )
}

/// The events that only change what is published.
fn bookkeeping(state: Companion, event: CompanionEvent) -> Companion {
    match event {
        CompanionEvent::Asked { task, prompt, .. } => turn_began(asked(state, task, prompt)),
        CompanionEvent::Answer(wire) => answer_moved(answered(state, *wire)),
        CompanionEvent::AnswerGone(task) => {
            let mut answers = state.answers;
            answers.remove(&task);
            let front = state.front.filter(|front| *front != task);
            Companion {
                answers,
                front,
                ..state
            }
        }
        CompanionEvent::CardPressed { task, action } => {
            with_answer(state, &task, |answer| match card_call(&answer, &action) {
                Some(step) => AnswerFacts {
                    busy: Busy::Acting { card: action, step },
                    ..answer
                },
                None => answer,
            })
        }
        CompanionEvent::CardRefused { task, action } => {
            with_answer(state, &task, |answer| AnswerFacts {
                busy: Busy::Refused(action),
                ..answer
            })
        }
        CompanionEvent::Shown { task, handle, text } => with_answer(state, &task, |answer| {
            let mut shown = answer.shown;
            shown.insert(handle, text);
            AnswerFacts { shown, ..answer }
        }),
        CompanionEvent::Unshown { task, handle } => with_answer(state, &task, |answer| {
            let mut shown = answer.shown;
            shown.insert(handle, handle_fallback(handle));
            AnswerFacts { shown, ..answer }
        }),
        CompanionEvent::Session { task, session } => {
            with_answer(state, &task, |answer| AnswerFacts {
                session: session.or(answer.session),
                ..answer
            })
        }
        CompanionEvent::Routing { task, notes } => {
            with_answer(state, &task, |answer| AnswerFacts {
                routing: notes,
                ..answer
            })
        }
        CompanionEvent::Run(run) => {
            let mut runs = state.runs;
            runs.insert(run.run.clone(), *run);
            Companion { runs, ..state }
        }
        CompanionEvent::RunGone(run) => {
            let mut runs = state.runs;
            runs.remove(&run);
            Companion { runs, ..state }
        }
        CompanionEvent::Sight(sight) => Companion { sight, ..state },
        CompanionEvent::Journal(journal) => Companion { journal, ..state },
        CompanionEvent::Openers(openers) => Companion { openers, ..state },
        CompanionEvent::ThingsAsked(generation) => Companion {
            things: state.things.asked(generation),
            ..state
        },
        CompanionEvent::ThingsFound(generation, hits) => Companion {
            things: state.things.answered(generation, hits),
            ..state
        },
        CompanionEvent::ThingsLate(generation, hits) => Companion {
            things: state.things.extended(generation, hits, same_thing),
            ..state
        },
        CompanionEvent::ThingsFailed(generation) => Companion {
            things: state.things.failed(generation),
            ..state
        },
        CompanionEvent::TimelineAsked(generation) => Companion {
            memory: state.memory.asked(generation),
            waiting: state.waiting.asked(generation),
            ..state
        },
        CompanionEvent::WaitingLoaded(generation, rows) => Companion {
            waiting: state.waiting.answered(generation, rows),
            ..state
        },
        CompanionEvent::WaitingFailed(generation) => Companion {
            waiting: state.waiting.failed(generation),
            ..state
        },
        CompanionEvent::Settled(fact) => {
            let found = state
                .waiting
                .found
                .iter()
                .filter(|view| view.fact.id != fact)
                .cloned()
                .collect();
            Companion {
                waiting: Lookup {
                    found,
                    ..state.waiting
                },
                memory_notice: None,
                memory_answers: state.memory_answers + 1,
                ..state
            }
        }
        CompanionEvent::SettleFailed => Companion {
            memory_notice: Some(SETTLE_FAILED.to_owned()),
            memory_answers: state.memory_answers + 1,
            ..state
        },
        CompanionEvent::TimelineLoaded(generation, rows) => Companion {
            memory: state.memory.answered(generation, rows),
            ..state
        },
        CompanionEvent::TimelineFailed(generation) => Companion {
            memory: state.memory.failed(generation),
            ..state
        },
        // The machines' events are handled by `step`.
        CompanionEvent::Link(_)
        | CompanionEvent::Tapped(_)
        | CompanionEvent::Summoned(_)
        | CompanionEvent::AppAnswered(..)
        | CompanionEvent::Elapsed
        | CompanionEvent::LauncherClosed
        | CompanionEvent::Dismissed
        | CompanionEvent::Helpers(_)
        | CompanionEvent::Terminals(_)
        | CompanionEvent::Mode(_) => state,
    }
}

fn same_thing(a: &Hit, b: &Hit) -> bool {
    a.entity.id == b.entity.id
}

/// An ask began: the answer exists, thinking, until the companion writes the first view.
fn asked(state: Companion, task: TaskId, prompt: String) -> Companion {
    let mut answers = state.answers;
    let known = answers.remove(&task).unwrap_or_else(|| AnswerFacts {
        prompt: String::new(),
        wire: thinking(&task),
        busy: Busy::Idle,
        session: None,
        routing: Vec::new(),
        shown: BTreeMap::new(),
    });
    answers.insert(
        task.clone(),
        AnswerFacts {
            prompt,
            busy: Busy::Idle,
            ..known
        },
    );
    Companion {
        answers,
        front: Some(task),
        ..state
    }
}

/// An answer as the companion wrote it: kept with the words that asked for it, if known.
fn answered(state: Companion, wire: AnswerWire) -> Companion {
    let mut answers = state.answers;
    let task = wire.task.clone();
    let known = answers.remove(&task);
    let facts = match known {
        Some(known) => AnswerFacts {
            busy: settled(known.busy, &wire),
            wire,
            ..known
        },
        None => AnswerFacts {
            prompt: String::new(),
            wire,
            busy: Busy::Idle,
            session: None,
            routing: Vec::new(),
            shown: BTreeMap::new(),
        },
    };
    answers.insert(task.clone(), facts);
    Companion {
        answers,
        front: state.front.or(Some(task)),
        ..state
    }
}

/// The answer before the companion has written one.
fn thinking(task: &TaskId) -> AnswerWire {
    AnswerWire {
        task: task.clone(),
        phase: AnswerPhase::Thinking,
        body: AnswerBody::Text { lines: Vec::new() },
        progress: Vec::new(),
        footer: FooterWire {
            served: Vec::new(),
            sources: Vec::new(),
            keep: ContextKeep {
                query: Keep::Dropped,
                results: Keep::Dropped,
                selection: Keep::Dropped,
                window: Keep::Dropped,
            },
        },
    }
}

fn with_answer(
    state: Companion,
    task: &TaskId,
    change: impl FnOnce(AnswerFacts) -> AnswerFacts,
) -> Companion {
    let mut answers = state.answers;
    if let Some(answer) = answers.remove(task) {
        answers.insert(task.clone(), change(answer));
    }
    Companion { answers, ..state }
}

impl<T: Clone> Lookup<T> {
    /// A newer ask replaces the lookup; an older one is ignored.
    pub fn asked(self, generation: Generation) -> Lookup<T> {
        if generation >= self.generation {
            Lookup {
                generation,
                phase: LookupPhase::Asked,
                found: Vec::new(),
            }
        } else {
            self
        }
    }

    /// The answer of the ask in flight.
    pub fn answered(self, generation: Generation, found: Vec<T>) -> Lookup<T> {
        if generation == self.generation {
            Lookup {
                phase: LookupPhase::Answered,
                found,
                ..self
            }
        } else {
            self
        }
    }

    /// More found for the ask in flight, those `same` as one already found left out.
    pub fn extended(
        self,
        generation: Generation,
        more: Vec<T>,
        same: impl Fn(&T, &T) -> bool,
    ) -> Lookup<T> {
        if generation != self.generation {
            return self;
        }
        let fresh: Vec<T> = more
            .into_iter()
            .filter(|new| !self.found.iter().any(|old| same(old, new)))
            .collect();
        Lookup {
            found: self.found.into_iter().chain(fresh).collect(),
            phase: LookupPhase::Answered,
            ..self
        }
    }

    /// The ask in flight failed.
    pub fn failed(self, generation: Generation) -> Lookup<T> {
        if generation == self.generation {
            Lookup {
                phase: LookupPhase::Failed,
                ..self
            }
        } else {
            self
        }
    }
}
