//! The clipboard history as launcher rows (quire design/09 H4: what you copied is one keystroke
//! away). The history is the clipboard service's; this provider only reads it.
//!
//! `query` answers with [`clip_items`]: typed text matches entries' text (images match "image" and
//! their type), nothing typed lists nothing (the empty launcher shows Actions and Recent, and
//! clipboard rows are never Recent); asked alone (the Clipboard filter, `Scope::Only`) it lists the
//! newest first. Enter makes the entry the clipboard again (`Activation::SetClipboard`); "Forget"
//! (⌘⌫, `Activation::ForgetClipboard`) drops it and the launcher stays open.

use std::time::{SystemTime, UNIX_EPOCH};

use nucleo_matcher::Matcher;
use sill_launcher::{
    Activation, Batch, Choice, Item, Needle, Provider, ProviderKind, Query, ResultSink, Scope,
    Subject, UnixTime, matching,
};
use tokio::sync::watch;

use sill_model::clipboard::Clipboard;

mod rows;

use rows::rows_at;

/// The action id that forgets an entry.
pub const FORGET_ACTION: &str = "forget";

/// Rows from the clipboard history.
#[derive(Debug, Clone)]
pub struct ClipboardProvider {
    pub history: watch::Receiver<Clipboard>,
}

impl Provider for ClipboardProvider {
    fn kind(&self) -> ProviderKind {
        ProviderKind::Clipboard
    }

    fn query(&self, query: &Query, sink: &mut dyn ResultSink) {
        if query.is_cancelled() {
            return;
        }
        let needle = Needle::parse(&query.text);
        let alone = query.scope == Scope::Only(ProviderKind::Clipboard);
        let items = if needle.is_empty() && !alone {
            Vec::new()
        } else {
            let history = self.history.borrow().clone();
            clip_items(&history, &needle, &mut matching::matcher())
        };
        if query.is_cancelled() {
            return;
        }
        sink.push(Batch {
            provider: ProviderKind::Clipboard,
            generation: query.generation,
            items,
        });
    }

    fn activate(&self, item: &Item, choice: &Choice) -> Activation {
        match (&item.subject, choice) {
            (Subject::Clip(clip), Choice::Primary) => Activation::SetClipboard { entry: clip.key },
            (Subject::Clip(clip), Choice::Action(id)) if id.0 == FORGET_ACTION => {
                Activation::ForgetClipboard { entry: clip.key }
            }
            _ => Activation::Nothing,
        }
    }
}

/// `history`'s entries as rows for `needle`, best first (newest first among equals). A text
/// row's title is its first line, trimmed to 80 characters; its subtitle its age and length
/// ("2 min ago · 1,204 characters"); an image row's title "Image", its subtitle its size and
/// type. Every row's subject is `Subject::Clip` with an excerpt, never the whole text.
/// Reads the clock for the ages; the private `rows::rows_at` is the pure part.
pub fn clip_items(history: &Clipboard, needle: &Needle, matcher: &mut Matcher) -> Vec<Item> {
    rows_at(history, needle, matcher, unix_now())
}

/// The wall clock, in whole seconds; a clock before 1970 reads as 1970 (every entry is then
/// "just now").
fn unix_now() -> UnixTime {
    UnixTime(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_secs()),
    )
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use sill_launcher::{Activation, Choice, Provider};
    use sill_launcher::{
        CharCount, ClipFacts, ClipKey, ClipKind, Excerpt, IconRef, Item, ItemId, Needle,
        ProviderKind, Score, Subject, UnixTime, matching::matcher,
    };
    use tokio::sync::watch;

    use super::{ClipboardProvider, clip_items};
    use sill_model::clipboard::{ClipContent, ClipEntry, ClipText, Clipboard};

    #[test]
    fn enter_makes_the_entry_the_clipboard_again() {
        let provider = ClipboardProvider {
            history: watch::channel(Clipboard::default()).1,
        };
        let row = Item {
            id: ItemId("7".to_owned()),
            provider: ProviderKind::Clipboard,
            title: "hello".to_owned(),
            subtitle: String::new(),
            icon: IconRef::None,
            score: Score(0),
            actions: Vec::new(),
            subject: Subject::Clip(ClipFacts {
                key: ClipKey(7),
                kind: ClipKind::Text {
                    excerpt: Excerpt("hello".to_owned()),
                    chars: CharCount(5),
                },
                copied: UnixTime(0),
            }),
        };
        assert_eq!(
            provider.activate(&row, &Choice::Primary),
            Activation::SetClipboard { entry: ClipKey(7) }
        );
    }

    #[test]
    fn typed_text_finds_entries_newest_first() {
        let entry = |key, text: &str, at| ClipEntry {
            key: ClipKey(key),
            content: ClipContent::Text(ClipText(Arc::from(text))),
            copied: UnixTime(at),
        };
        let history = Clipboard {
            entries: vec![
                entry(3, "fn main() {}", 30),
                entry(2, "unrelated", 20),
                entry(1, "fn main() {}\n// older copy", 10),
            ],
            ..Clipboard::default()
        };
        let rows = clip_items(&history, &Needle::parse("main"), &mut matcher());
        let keys: Vec<ClipKey> = rows
            .iter()
            .filter_map(|r| match &r.subject {
                Subject::Clip(c) => Some(c.key),
                _ => None,
            })
            .collect();
        assert_eq!(keys, [ClipKey(3), ClipKey(1)]);
        assert_eq!(rows[1].title, "fn main() {}", "the first line");
    }
}
