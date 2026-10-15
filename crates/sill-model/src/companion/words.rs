//! The words a companion row wears in the launcher: fixed words and the person's own, never a
//! model's formatting. Each returns a title and a faint second line.

use almanac_core::{EntryBody, TimelineEntry};
use companion_wire::{AnswerBody, AnswerPhase, NeedsYou, RefusalWire, StepWireState};
use docket_core::{Reveal, UndoEntry, UndoState};
use prov::Actor;

use super::emphasis::plain_of;
use super::model::{AnswerFacts, Busy};
use super::model_picker::lacks_model;
use super::program_labels::ProgramLabels;
use super::progress::row_progress;

/// The answer's row: what was asked, and where the answer stands.
pub fn answer_line(answer: &AnswerFacts) -> (String, String) {
    let title = match answer.prompt.trim() {
        "" => "Companion".to_owned(),
        asked => asked.to_owned(),
    };
    (title, standing(answer))
}

/// Where the answer stands: what it waits for, how it failed, or its latest line. A pressed
/// button's step that is still going, or that failed, is said even when the task itself is
/// done. A turn that met no model says nothing: the model picker opens instead.
fn standing(answer: &AnswerFacts) -> String {
    if lacks_model(&answer.wire.body) {
        return String::new();
    }
    match &answer.wire.phase {
        AnswerPhase::Thinking => row_progress(answer).unwrap_or_else(|| "Thinking".to_owned()),
        AnswerPhase::Streaming => row_progress(answer)
            .or_else(|| last_plain_line(answer))
            .unwrap_or_else(|| "Writing".to_owned()),
        AnswerPhase::NeedsYou(needs) => match needs {
            NeedsYou::Confirm(_) => "Needs your confirmation".to_owned(),
            NeedsYou::Form(_) => "Needs a few details".to_owned(),
            NeedsYou::Question { text, .. } => plain_of(text),
        },
        AnswerPhase::Done => done_words(answer),
        AnswerPhase::Failed => refusal_words(&answer.wire.body),
        AnswerPhase::Cancelled => "Stopped".to_owned(),
    }
}

/// A finished task's line: its plan's trouble first, then a press still going, then the text.
fn done_words(answer: &AnswerFacts) -> String {
    let steps = match &answer.wire.body {
        AnswerBody::Plan(plan) => plan.steps.as_slice(),
        _ => &[],
    };
    let failed = steps
        .iter()
        .any(|step| matches!(step.state, StepWireState::Failed(_)));
    let going = steps
        .iter()
        .any(|step| matches!(step.state, StepWireState::Pending | StepWireState::Running));
    match (&answer.busy, failed, going) {
        (_, true, _) => "A step failed".to_owned(),
        (Busy::Refused(_), false, _) => "Could not do that".to_owned(),
        (Busy::Acting { .. }, false, _) | (Busy::Idle, false, true) => "Working on it".to_owned(),
        (Busy::Idle, false, false) => last_plain_line(answer).unwrap_or_else(|| "Done".to_owned()),
    }
}

/// The lines of a text answer, trimmed and with the empty ones left out, marks and all; a
/// handle's line is the text the router showed for it. Other bodies have none.
pub fn body_lines(answer: &AnswerFacts) -> Vec<String> {
    match &answer.wire.body {
        AnswerBody::Text { lines } => lines
            .iter()
            .filter_map(|line| match line {
                Reveal::Plain(text) => Some(text.as_str()),
                Reveal::Handle(handle) => answer.shown.get(handle).map(String::as_str),
            })
            .map(str::trim)
            .filter(|text| !text.is_empty())
            .map(str::to_owned)
            .collect(),
        _ => Vec::new(),
    }
}

/// The last line of a text answer as words alone: the answer itself once finished (the lines
/// before it say what the companion was doing), the latest progress while it is written.
fn last_plain_line(answer: &AnswerFacts) -> Option<String> {
    body_lines(answer).last().map(|line| plain_of(line))
}

/// Why an answer failed, in the person's terms.
fn refusal_words(body: &AnswerBody) -> String {
    match body {
        AnswerBody::Refused(RefusalWire::NeedsCloud(_)) => "Needs a cloud model".to_owned(),
        AnswerBody::Refused(RefusalWire::NotAllowed(_)) => "Not allowed".to_owned(),
        AnswerBody::Refused(RefusalWire::NoWay { .. }) => "No way to do that".to_owned(),
        AnswerBody::Refused(RefusalWire::OverBudget(_)) => "Over its budget".to_owned(),
        AnswerBody::Refused(RefusalWire::Failed(why)) => why.clone(),
        AnswerBody::Refused(RefusalWire::NoModel) => String::new(),
        _ => "Could not finish".to_owned(),
    }
}

/// A journal row: what was done in the app's own words, who did it and whether it can still be
/// undone.
pub fn activity_line(entry: &UndoEntry) -> (String, String) {
    let who = match &entry.actor {
        Actor::User { .. } => "You",
        Actor::Companion { .. } => "Companion",
        Actor::Mcp { .. } => "An outside assistant",
        Actor::Acp { .. } => "An assistant",
        Actor::Cli => "A terminal",
        Actor::App { .. } | Actor::ThirdParty { .. } => "An app",
        Actor::System { .. } => "The system",
        Actor::Unknown => "Something",
    };
    let standing = match &entry.state {
        UndoState::Available => String::new(),
        UndoState::Undoing => "Undoing".to_owned(),
        UndoState::Undone { .. } => "Undone".to_owned(),
        UndoState::Failed(_) => "Could not be undone".to_owned(),
        UndoState::Expired => "Can no longer be undone".to_owned(),
    };
    let subtitle = match standing.as_str() {
        "" => who.to_owned(),
        said => format!("{who} · {said}"),
    };
    (entry.said.as_str().to_owned(), subtitle)
}

/// A timeline row: who did what to which thing, and in which app.
pub fn memory_line(entry: &TimelineEntry, programs: &ProgramLabels) -> (String, String) {
    let who = match &entry.actor {
        Actor::User { .. } => "You".to_owned(),
        Actor::Companion { .. } => "Companion".to_owned(),
        Actor::Mcp { .. } => "An outside assistant".to_owned(),
        Actor::Acp { program, label } => match programs.name_of(program.as_str(), label.as_ref()) {
            Some(name) => format!("{name} (assistant)"),
            None => "An assistant".to_owned(),
        },
        Actor::Cli => "A terminal".to_owned(),
        Actor::App { app } | Actor::ThirdParty { app, .. } => last_element(app.as_str()),
        Actor::System { .. } => "The system".to_owned(),
        Actor::Unknown => "Something".to_owned(),
    };
    let verb = entry
        .kind
        .as_str()
        .rsplit('.')
        .next()
        .unwrap_or_default()
        .replace('_', " ");
    let thing = match entry.body {
        EntryBody::Present => entry.things.first(),
        // Erased, or a body kind this build does not know: no thing to name.
        _ => None,
    };
    let title = match thing {
        Some(thing) => format!("{who} {verb} {}", thing.title.as_str()),
        None => format!("{who} {verb}"),
    };
    let app = thing
        .map(|thing| last_element(thing.thing.app.as_str()))
        .unwrap_or_default();
    (title, app)
}

fn last_element(name: &str) -> String {
    name.rsplit('.').next().unwrap_or(name).to_owned()
}
