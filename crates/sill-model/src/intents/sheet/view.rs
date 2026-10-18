//! What the sheet shows of the question on screen, as plain words: pure, so the surface only
//! draws. The title is the manifest's label of the action (the app's own words), the byline names
//! the asker and the app, the facts are the arguments and the concrete change as the router
//! formatted them, and the notes are the router's reasons and what the session read.

use docket_core::{
    ConfirmDetail, ConfirmId, ConfirmOffer, ConfirmRequest, Gesture, LimitNote, NeverOffer,
    StandingGrantId, TaintNote, UndoOffer,
};
use prov::Effect;

use super::brief::BriefView;
use super::brief_wire::brief_of;
use super::model::{Pending, SHEET_ROWS_MAX, SHEET_VALUE_MAX_CHARS, Sheets};
use super::words::{
    app_words, asker_words, effect_words, origin_words, shown_words, source_words, why_words,
};
use crate::untrusted::for_display;

/// How many reasons the sheet gives.
const SHEET_NOTES_MAX: usize = 8;

/// One line of the sheet: what it is, what it says, and for somebody else's words where they
/// came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SheetFact {
    pub label: String,
    pub value: String,
    pub source: Option<String>,
}

/// The limit that made the sheet appear, as the sheet says it: the person's own words or a plain
/// reason, and the entry Undo takes away.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SheetHeld {
    /// The entry Undo removes.
    pub id: StandingGrantId,
    /// You said: “don't touch Work”
    pub why: String,
    /// Whether the entry may be undone here (an administrator's may not).
    pub undo: UndoOffer,
}

impl From<&LimitNote> for SheetHeld {
    fn from(note: &LimitNote) -> SheetHeld {
        SheetHeld {
            id: note.id.clone(),
            why: note.why.clone(),
            undo: note.undo,
        }
    }
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
    /// Whether "Never" is a button beside the allow choices.
    pub never: NeverOffer,
    /// The limit that made it ask, when one did.
    pub held: Option<SheetHeld>,
    pub gesture: Gesture,
    pub effect: Effect,
    /// How many more questions wait behind this one.
    pub waiting: usize,
    /// The editable brief, when this is the sheet that checks one: it replaces the facts, the
    /// notes and the allow choices.
    pub brief: Option<BriefView>,
}

/// The question on screen, if there is one.
pub fn sheet_view(sheets: &Sheets) -> Option<SheetView> {
    sheets
        .current()
        .map(|pending| view_of(pending, sheets.waiting()))
}

fn view_of(pending: &Pending, waiting: usize) -> SheetView {
    let request = &pending.request;
    let brief = brief_of(request);
    SheetView {
        id: request.id.clone(),
        title: title_of(request, brief.as_ref()),
        byline: byline_of(request, brief.as_ref()),
        facts: facts_of(request),
        notes: notes_of(request),
        offer: request.offer,
        never: request.limits.never,
        held: request.limits.held.as_ref().map(SheetHeld::from),
        gesture: request.gesture,
        effect: request.effect,
        waiting,
        brief,
    }
}

/// The action's own label as a question, or for a brief the words that ask for a check.
fn title_of(request: &ConfirmRequest, brief: Option<&BriefView>) -> String {
    match brief {
        Some(_) => "Check this before helpers use it".to_owned(),
        None => format!(
            "{}?",
            for_display(request.action.as_str(), SHEET_VALUE_MAX_CHARS)
        ),
    }
}

/// Who asks and where, or for a brief where it came from.
fn byline_of(request: &ConfirmRequest, brief: Option<&BriefView>) -> String {
    match brief {
        Some(_) => origin_words(request),
        None => format!(
            "{} wants to do this in {}.",
            asker_of(request),
            app_words(&request.app)
        ),
    }
}

/// Who asks: the helper by the name the person gave it, else the kind of caller.
fn asker_of(request: &ConfirmRequest) -> String {
    match &request.helper {
        Some(name) => for_display(name.as_str(), SHEET_VALUE_MAX_CHARS),
        None => asker_words(&request.actor).to_owned(),
    }
}

fn fact(label: &str, value: String, source: Option<String>) -> SheetFact {
    SheetFact {
        label: label.to_owned(),
        value,
        source,
    }
}

/// The first [`SHEET_ROWS_MAX`] of `total` rows, then one under `label` that counts the rest.
fn capped(rows: impl Iterator<Item = SheetFact>, total: usize, label: &str) -> Vec<SheetFact> {
    let mut shown: Vec<SheetFact> = rows.take(SHEET_ROWS_MAX).collect();
    if total > SHEET_ROWS_MAX {
        let more = total - SHEET_ROWS_MAX;
        shown.push(fact(label, format!("and {more} more"), None));
    }
    shown
}

/// The arguments, then the concrete change.
fn facts_of(request: &ConfirmRequest) -> Vec<SheetFact> {
    let lines = request.lines.iter().map(|line| {
        let (value, source) = shown_words(&line.value);
        fact(line.label.as_str(), value, source)
    });
    let mut facts = capped(lines, request.lines.len(), "More");
    if request.count.0 > 1 {
        facts.push(fact("How many", request.count.0.to_string(), None));
    }
    facts.extend(detail_facts(&request.detail));
    facts
}

fn detail_facts(detail: &ConfirmDetail) -> Vec<SheetFact> {
    match detail {
        ConfirmDetail::Plain => Vec::new(),
        ConfirmDetail::Recipients(who) => {
            let rows = who.iter().map(|shown| {
                let (value, source) = shown_words(shown);
                fact("To", value, source)
            });
            capped(rows, who.len(), "To")
        }
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
        ConfirmDetail::Entities(things) => {
            let rows = things.iter().map(|thing| {
                fact(
                    "Affects",
                    format!(
                        "\u{201c}{}\u{201d}",
                        for_display(&thing.title.value, SHEET_VALUE_MAX_CHARS)
                    ),
                    None,
                )
            });
            capped(rows, things.len(), "Affects")
        }
        ConfirmDetail::Preview(_) => Vec::new(),
    }
}

/// The reasons, then what the session read; the effect only when no reason already said it.
fn notes_of(request: &ConfirmRequest) -> Vec<String> {
    let mut notes: Vec<String> = request
        .why
        .iter()
        .take(SHEET_NOTES_MAX)
        .map(why_words)
        .collect();
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
