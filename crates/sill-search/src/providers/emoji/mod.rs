//! Emoji by name, keyword and CLDR short name, with a recent-first grid and the person's skin tone
//! (`launcher.emoji_skin_tone`).
//!
//! `query` answers with [`search::emoji_items`] over [`table::emoji_table`]: typed text of two or
//! more characters (or any text after `:`) lists matches; nothing typed lists nothing for the whole
//! launcher, and the recent-first grid when the Emoji filter asks this provider alone
//! (`Query::scope == Scope::Only(Emoji)`). Enter copies the emoji (`Activation::Copy`); the
//! launcher cannot type into another app.

pub mod search;
pub mod table;
pub mod words;

use std::sync::{Arc, Mutex};

use sill_launcher::{
    Activation, Batch, Choice, Frecency, Item, Needle, Provider, ProviderKind, Query, ResultSink,
    Scope, Subject, matching,
};
use tokio::sync::watch;

use crate::clock::unix_now;
use crate::providers::locked::copy_of;

use sill_model::results::LauncherPolicy;

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
                // Nothing typed, asked of the whole launcher: no emoji rows.
                return;
            }
        } else if !worth_searching(trimmed) {
            return;
        }

        let needle = Needle::parse(&query.text);
        let tone = self.policy.borrow().emoji_tone;
        let frecency = copy_of(&self.frecency);
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

/// Rows a query may answer with: `SectionPlan::default_plan().ceiling`, the most any section
/// publishes; capping to the grid's own display cap and revealing more on "Show More" is left to
/// `rank::sections::place`.
const EMOJI_LIMIT: usize = 20;

/// Typed text is worth searching at two or more characters, or any text after a leading `:`
/// (this module's doc); one bare character (or a bare `:`) is not.
fn worth_searching(trimmed: &str) -> bool {
    match trimmed.strip_prefix(':') {
        Some(rest) => !rest.trim().is_empty(),
        None => trimmed.chars().count() >= 2,
    }
}
