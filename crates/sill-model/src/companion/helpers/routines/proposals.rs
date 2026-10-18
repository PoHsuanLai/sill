//! The words a suggested routine wears. A helper (or the agent app behind it) may suggest a
//! routine; sill shows what it would do and when, and makes nothing until the person accepts. The
//! suggester's own reason is shown as a quotation, never as sill's words.

use companion_wire::{RoutineProposal, WhenForm};

/// Over a suggestion.
pub const PROPOSAL_TITLE: &str = "Suggested routine";

/// Before the reason the suggester gave, which is quoted.
pub const PROPOSAL_WHY_LEAD: &str = "Its reason:";

/// The button that makes the routine.
pub const PROPOSAL_ACCEPT: &str = "Accept";

/// The button that leaves it for now.
pub const PROPOSAL_DECLINE: &str = "Not Now";

/// What the routine would do, as the suggestion states it.
pub fn proposal_what(proposal: &RoutineProposal) -> &str {
    proposal.form.goal.as_str()
}

/// When it would run, in plain words.
pub fn proposal_when(proposal: &RoutineProposal) -> String {
    match &proposal.form.when {
        WhenForm::At(text) => at_words(text),
        WhenForm::On(event) => event.words.clone(),
        WhenForm::AfterJob(_) => "After another job is done".to_owned(),
    }
}

/// The suggester's reason, in quotation marks: it is data, not something sill says. Nothing for
/// an empty reason.
pub fn proposal_why(proposal: &RoutineProposal) -> Option<String> {
    let why = proposal.why.trim();
    (!why.is_empty()).then(|| format!("\u{201c}{why}\u{201d}"))
}

/// "daily 08:00" as "Every day at 08:00", "weekdays 08:00" as "Weekdays at 08:00" and "every 2
/// hours" as it is with a capital; a time written some other way is shown as it was written.
fn at_words(text: &str) -> String {
    let text = text.trim();
    match text.split_once(' ') {
        Some(("daily", time)) => format!("Every day at {time}"),
        Some(("weekdays", time)) => format!("Weekdays at {time}"),
        _ => {
            let mut chars = text.chars();
            chars
                .next()
                .map(|first| first.to_uppercase().chain(chars).collect())
                .unwrap_or_default()
        }
    }
}
