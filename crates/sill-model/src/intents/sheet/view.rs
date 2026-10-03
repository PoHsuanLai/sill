//! What the sheet shows of the question on screen, as plain words: pure, so the surface only
//! draws. The title is the manifest's label of the action (the app's own words), the byline names
//! the asker and the app, the facts are the arguments and the concrete change as the router
//! formatted them, and the notes are the router's reasons and what the session read.

use docket_core::{ConfirmDetail, ConfirmId, ConfirmOffer, ConfirmRequest, Gesture, TaintNote};
use prov::Effect;

use super::model::{Pending, Sheets};
use super::words::{app_words, asker_words, effect_words, shown_words, source_words, why_words};

/// One line of the sheet: what it is, what it says, and for somebody else's words where they
/// came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SheetFact {
    pub label: String,
    pub value: String,
    pub source: Option<String>,
}

/// The question on screen, as the surface draws it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SheetView {
    pub id: ConfirmId,
    /// "Archive a thread?"
    pub title: String,
    /// "The terminal wants to do this in Mail."
    pub byline: String,
    pub facts: Vec<SheetFact>,
    /// The reasons it asks and what the session read, each a short sentence.
    pub notes: Vec<String>,
    pub offer: ConfirmOffer,
    pub gesture: Gesture,
    pub effect: Effect,
    /// How many more questions wait behind this one.
    pub waiting: usize,
}

/// The question on screen, if there is one.
pub fn sheet_view(sheets: &Sheets) -> Option<SheetView> {
    sheets
        .current()
        .map(|pending| view_of(pending, sheets.waiting()))
}

fn view_of(pending: &Pending, waiting: usize) -> SheetView {
    let request = &pending.request;
    SheetView {
        id: request.id.clone(),
        title: format!("{}?", request.action.as_str()),
        byline: format!(
            "{} wants to do this in {}.",
            asker_words(&request.actor),
            app_words(&request.app)
        ),
        facts: facts_of(request),
        notes: notes_of(request),
        offer: request.offer,
        gesture: request.gesture,
        effect: request.effect,
        waiting,
    }
}

fn fact(label: &str, value: String, source: Option<String>) -> SheetFact {
    SheetFact {
        label: label.to_owned(),
        value,
        source,
    }
}

/// The arguments, then the concrete change.
fn facts_of(request: &ConfirmRequest) -> Vec<SheetFact> {
    let lines = request.lines.iter().map(|line| {
        let (value, source) = shown_words(&line.value);
        fact(line.label.as_str(), value, source)
    });
    let mut facts: Vec<SheetFact> = lines.collect();
    if request.count.0 > 1 {
        facts.push(fact("How many", request.count.0.to_string(), None));
    }
    facts.extend(detail_facts(&request.detail));
    facts
}

fn detail_facts(detail: &ConfirmDetail) -> Vec<SheetFact> {
    match detail {
        ConfirmDetail::Plain => Vec::new(),
        ConfirmDetail::Recipients(who) => who
            .iter()
            .map(|shown| {
                let (value, source) = shown_words(shown);
                fact("To", value, source)
            })
            .collect(),
        ConfirmDetail::Destination(shown) => {
            let (value, source) = shown_words(shown);
            vec![fact("To", value, source)]
        }
        ConfirmDetail::TextDiff { before, after } => {
            let (was, was_from) = shown_words(before);
            let (now, now_from) = shown_words(after);
            vec![fact("Now", was, was_from), fact("Becomes", now, now_from)]
        }
        ConfirmDetail::Moves(moves) => vec![fact("Moves", format!("{} files", moves.len()), None)],
        ConfirmDetail::Entities(things) => things
            .iter()
            .map(|thing| {
                fact(
                    "Affects",
                    format!("\u{201c}{}\u{201d}", thing.title.value),
                    None,
                )
            })
            .collect(),
        ConfirmDetail::Preview(_) => Vec::new(),
    }
}

/// The reasons, then what the session read; the effect only when no reason already said it.
fn notes_of(request: &ConfirmRequest) -> Vec<String> {
    let mut notes: Vec<String> = request.why.iter().map(why_words).collect();
    if let TaintNote::ReadUntrusted(sources) = &request.taint {
        let from: Vec<&str> = sources.iter().map(source_words).collect();
        notes.push(format!("It read something from {}.", from.join(", ")));
    }
    let said = effect_words(request.effect);
    if !notes.iter().any(|note| note == said) {
        notes.push(said.to_owned());
    }
    notes
}
