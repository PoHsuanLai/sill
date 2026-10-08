//! The order the provider list is handed to the drawn list in, for what has been typed in the
//! search: the label that is what was typed first, then the labels that start with it, then the
//! rest in their own order. Return picks the first row, so the person who typed "Google" gets
//! Google, not the "Google (Gemini)" key provider that happens to be listed above it.

use ds_shell::accounts::model::ProviderEntry;

/// How well a label answers the search: smaller is better.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Match {
    Exact,
    Prefix,
    Inside,
}

fn match_of(label: &str, wanted: &str) -> Match {
    let label = label.to_lowercase();
    match (label == wanted, label.starts_with(wanted)) {
        (true, _) => Match::Exact,
        (false, true) => Match::Prefix,
        (false, false) => Match::Inside,
    }
}

/// `entries` ranked for `query` (case and the blanks around it ignored). Entries of one rank keep
/// their order, and an empty query changes nothing.
pub(super) fn ranked(mut entries: Vec<ProviderEntry>, query: &str) -> Vec<ProviderEntry> {
    let wanted = query.trim().to_lowercase();
    entries.sort_by_key(|entry| match_of(&entry.label, &wanted));
    entries
}
