//! Emoji by name, keyword and CLDR short name (M9 lane c, FINDINGS "M9 freeze" F649), with a
//! recent-first grid and the person's skin tone (`launcher.emoji_skin_tone`).
//!
//! The freeze's `query` answers nothing; lane c fills it with [`search::emoji_items`] over
//! [`table::emoji_table`]: typed text of two or more characters (or any text after `:`) lists
//! matches; nothing typed lists nothing for the whole launcher, and the recent-first grid when
//! the Emoji filter asks this provider alone (`Query::scope == Scope::Only(Emoji)`, F651).
//! Enter copies the emoji (`Activation::Copy`); the launcher cannot type into another app.

pub mod search;
pub mod table;

use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use sill_launcher::{
    Activation, Batch, Choice, Frecency, Item, Needle, Provider, ProviderKind, Query, ResultSink,
    Scope, Subject, UnixTime, matching,
};
use tokio::sync::watch;

use crate::results::LauncherPolicy;

/// The emoji provider.
#[derive(Debug, Clone)]
pub struct EmojiProvider {
    pub policy: watch::Receiver<LauncherPolicy>,
    /// The runner's history: recently chosen emoji lead.
    pub frecency: Arc<Mutex<Frecency>>,
}

impl Provider for EmojiProvider {
    fn kind(&self) -> ProviderKind {
        ProviderKind::Emoji
    }

    fn query(&self, query: &Query, sink: &mut dyn ResultSink) {
        if query.is_cancelled() {
            return;
        }
        let trimmed = query.text.trim();
        let asked_alone = query.scope == Scope::Only(ProviderKind::Emoji);
        if trimmed.is_empty() {
            if !asked_alone {
                // Nothing typed, asked of the whole launcher: no emoji rows (M9 freeze F651).
                return;
            }
        } else if !worth_searching(trimmed) {
            return;
        }

        let needle = Needle::parse(&query.text);
        let tone = self.policy.borrow().emoji_tone;
        let frecency = snapshot(&self.frecency);
        let now = unix_now();
        let mut matcher = matching::matcher();
        let items = search::emoji_items(
            table::emoji_table(),
            &needle,
            tone,
            &frecency,
            now,
            EMOJI_LIMIT,
            &mut matcher,
        );

        if query.is_cancelled() {
            return;
        }
        sink.push(Batch {
            provider: ProviderKind::Emoji,
            generation: query.generation,
            items,
        });
    }

    fn activate(&self, item: &Item, choice: &Choice) -> Activation {
        match (&item.subject, choice) {
            (Subject::Emoji(emoji), Choice::Primary) => Activation::Copy {
                text: emoji.glyph.clone(),
            },
            _ => Activation::Nothing,
        }
    }
}

/// Rows a query may answer with: `SectionPlan::default_plan().ceiling` (F645), the most any
/// section publishes; capping to the grid's own display cap and revealing more on "Show More"
/// is lane d's (`rank::sections::place`).
const EMOJI_LIMIT: usize = 20;

/// Typed text is worth searching at two or more characters, or any text after a leading `:`
/// (this module's doc); one bare character (or a bare `:`) is not.
fn worth_searching(trimmed: &str) -> bool {
    match trimmed.strip_prefix(':') {
        Some(rest) => !rest.trim().is_empty(),
        None => trimmed.chars().count() >= 2,
    }
}

/// A clone of the shared frecency, for one query.
fn snapshot(frecency: &Mutex<Frecency>) -> Frecency {
    frecency
        .lock()
        .map(|guard| guard.clone())
        .unwrap_or_else(|poisoned| poisoned.into_inner().clone())
}

/// The wall clock, in whole seconds since the epoch; a clock that cannot be read (never
/// observed, but not a reason to panic a keystroke) ages every entry from year 1970.
fn unix_now() -> UnixTime {
    UnixTime(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0),
    )
}
