//! The sheet's fixed words. Every one is the shell's own: who is asking, why it asks, what a thing
//! is, where a value came from. Nothing here is taken from a model or an app's prose, so a
//! sheet can never be made to say what the person did not ask.

use docket_core::{AskReason, Shown};
use prov::{Actor, AppName, Effect, Source};

/// Who is asking.
pub fn asker_words(actor: &Actor) -> &'static str {
    match actor {
        Actor::Cli => "The terminal",
        Actor::Companion { .. } => "The companion",
        Actor::Mcp { .. } => "An outside agent",
        Actor::User { .. } => "You",
        Actor::App { .. } | Actor::ThirdParty { .. } => "An app",
        Actor::System { .. } | Actor::Unknown => "Something on this computer",
    }
}

/// The app's name as people say it: the last element of its reverse-DNS name (`org.quire.Mail` is
/// "Mail").
pub fn app_words(app: &AppName) -> String {
    let name = app.as_str();
    name.rsplit('.').next().unwrap_or(name).to_owned()
}

/// What the action does to the world, in a few words.
pub fn effect_words(effect: Effect) -> &'static str {
    match effect {
        Effect::Read => "It only looks.",
        Effect::UndoableWrite => "It changes something you can undo.",
        Effect::Outbound => "It sends something out.",
        Effect::Destructive => "It cannot be undone.",
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
        AskReason::OutsideTask => "It goes beyond what you asked for.".to_owned(),
        AskReason::Disagreement => "The reviewers did not agree.".to_owned(),
        AskReason::Rule(_) => "A rule asks.".to_owned(),
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
        Source::Mcp(_) => "an outside agent",
        Source::Cli => "the terminal",
    }
}

/// A shown value as text and, for somebody else's words, where they came from. Their words are
/// drawn in quotes, as data.
pub fn shown_words(shown: &Shown) -> (String, Option<String>) {
    match shown {
        Shown::Plain(text) => (text.clone(), None),
        Shown::Quoted { text, from } => (
            format!("\u{201c}{text}\u{201d}"),
            Some(source_words(from).to_owned()),
        ),
    }
}
