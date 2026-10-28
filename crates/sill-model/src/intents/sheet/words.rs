//! The sheet's fixed words. Every one is the shell's own: who is asking, why it asks, what a thing
//! is, where a value came from. Nothing here is taken from a model or an app's prose, so a
//! sheet can never be made to say what the person did not ask.

use docket_core::{ConfirmRequest, Shown, TaintNote};
use docket_permissions_core::AskReason;
use prov::{Actor, AppName, Effect, Source};

use super::model::SHEET_VALUE_MAX_CHARS;
use crate::untrusted::for_display;

/// Who is asking.
pub fn asker_words(actor: &Actor) -> &'static str {
    match actor {
        Actor::Cli => "The terminal",
        Actor::Companion { .. } => "The assistant",
        Actor::Mcp { .. } => "An outside assistant",
        Actor::Acp { .. } => "An assistant",
        Actor::User { .. } => "You",
        Actor::App { .. } | Actor::ThirdParty { .. } => "An app",
        Actor::System { .. } | Actor::Unknown => "Something on this computer",
    }
}

/// The app's name as people say it: the last element of its reverse-DNS name (`org.quire.Mail` is
/// "Mail"). The assistant's own app is called by the person's word for it.
pub fn app_words(app: &AppName) -> String {
    let name = app.as_str();
    match name.rsplit('.').next().unwrap_or(name) {
        "Companion" | "Companion1" | "Companiond" => "Assistant".to_owned(),
        last => last.to_owned(),
    }
}

/// What the action does to the world, in a few words.
pub fn effect_words(effect: Effect) -> &'static str {
    match effect {
        Effect::Read => "It only looks.",
        Effect::UndoableWrite => "It changes something you can undo.",
        Effect::Outbound => "It sends something out.",
        Effect::Destructive => "It cannot be undone.",
        Effect::Execute => "It runs commands on this computer.",
    }
}

/// Why the router asks, one short phrase per reason.
pub fn why_words(reason: &AskReason) -> String {
    match reason {
        AskReason::Effect(effect) => effect_words(*effect).to_owned(),
        AskReason::Tainted => "It read something you may not trust.".to_owned(),
        AskReason::CrossSpace => "It acts in another Space.".to_owned(),
        AskReason::Mass(count) => format!("It touches {} things.", count.0),
        AskReason::FirstUse => "This is the first time for this app.".to_owned(),
        AskReason::FromTerminal => "It was typed in a terminal.".to_owned(),
        AskReason::AskAlways => "This action always asks.".to_owned(),
        AskReason::LastingFromUntrusted => {
            "It would remember something you did not write.".to_owned()
        }
        AskReason::RuleOfTwo => "Private data and an outside channel meet.".to_owned(),
        AskReason::UntrustedSink(_) => "Something you did not write feeds this.".to_owned(),
        AskReason::EvaluationFailed => {
            "Your rules could not be checked for this, so it asks you.".to_owned()
        }
        AskReason::OutsideTask => "It goes beyond what you asked for.".to_owned(),
        AskReason::Disagreement => "The reviewers did not agree.".to_owned(),
        AskReason::Rule(_) => "A rule asks.".to_owned(),
        // The rule's own sentence (its `@why`); a rule that wrote none reads like `Rule`.
        AskReason::Said { why, .. } if why.trim().is_empty() => "A rule asks.".to_owned(),
        AskReason::Said { why, .. } => why.clone(),
    }
}

/// Where a value came from, in a few words.
pub fn source_words(source: &Source) -> &'static str {
    match source {
        Source::User => "you",
        Source::App(_) => "an app",
        Source::Mail => "a message",
        Source::Web => "a web page",
        Source::File => "a file",
        Source::Screen { .. } => "the screen",
        Source::Clipboard => "the clipboard",
        Source::Calendar => "the calendar",
        Source::Contacts => "your contacts",
        Source::Notes => "your notes",
        Source::Model(_) => "a model",
        Source::Mcp(_) => "an outside assistant",
        Source::Cli => "the terminal",
    }
}

/// A shown value as text and, for somebody else's words, where they came from. Their words are
/// drawn in quotes, as data. Either way the text is one line of at most [`SHEET_VALUE_MAX_CHARS`]
/// characters with no control or bidirectional marks, so a value cannot pass for another row.
pub fn shown_words(shown: &Shown) -> (String, Option<String>) {
    match shown {
        Shown::Plain(text) => (for_display(text, SHEET_VALUE_MAX_CHARS), None),
        Shown::Quoted { text, from } => (
            format!(
                "\u{201c}{}\u{201d}",
                for_display(text, SHEET_VALUE_MAX_CHARS)
            ),
            Some(source_words(from).to_owned()),
        ),
    }
}

/// Where a brief came from: who wrote it and, from the router's note of what the session read,
/// what that was. The planner cannot change it.
pub fn origin_words(request: &ConfirmRequest) -> String {
    let writer = match &request.helper {
        Some(name) => for_display(name.as_str(), SHEET_VALUE_MAX_CHARS),
        None => asker_words(&request.actor).to_owned(),
    };
    match &request.taint {
        TaintNote::Clean => format!("{writer} wrote this."),
        TaintNote::ReadUntrusted(sources) => {
            let from: Vec<&str> = sources.iter().map(source_words).collect();
            format!("{writer} wrote this after reading {}.", from.join(" and "))
        }
    }
}
