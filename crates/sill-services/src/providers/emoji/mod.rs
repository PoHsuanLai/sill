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

use sill_launcher::{Activation, Choice, Frecency, Item, Provider, ProviderKind, Query, ResultSink, Subject};
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
        let _ = (query, sink);
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
