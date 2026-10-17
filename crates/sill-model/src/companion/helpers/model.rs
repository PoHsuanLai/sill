//! What the helpers screen holds: the list, the card being read or filled in, the sentences the
//! helpers told the person, and the one sentence the screen shows about the last thing that was
//! refused.

use std::collections::BTreeSet;

use companion_wire::{HelperCardView, HelperLine, ToldWire};
use prov::{AppName, HelperId};

use super::draft::Draft;
use super::models::ModelList;

/// A sentence a helper told the person, in plain words.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Told {
    /// Its number: later sentences have larger numbers.
    pub n: u64,
    /// The helper it is about.
    pub helper: HelperId,
    /// The sentence.
    pub text: String,
}

impl From<ToldWire> for Told {
    fn from(wire: ToldWire) -> Told {
        Told {
            n: wire.n,
            helper: wire.helper,
            text: wire.text,
        }
    }
}

/// A helper's card as read, with what the person has typed over it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenCard {
    /// The card as the companion has it.
    pub view: HelperCardView,
    /// The card as the person has it in the form.
    pub draft: Draft,
}

/// What the card area shows.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Open {
    /// No card.
    #[default]
    Nothing,
    /// This helper's card was asked for and has not arrived.
    Reading(HelperId),
    /// A helper's card.
    Card(Box<OpenCard>),
    /// A new helper's card, not yet saved.
    New(Draft),
}

/// Whether the person was asked to confirm removing a helper.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Removal {
    /// No question is open.
    #[default]
    Unasked,
    /// Asked "Remove it?" about this helper and waiting for the answer.
    Asked(HelperId),
}

/// Everything the helpers screen shows.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct HelpersModel {
    /// One line per helper, in the order the companion lists them.
    pub lines: Vec<HelperLine>,
    /// The sentences helpers told the person, oldest first, the newest few kept.
    pub told: Vec<Told>,
    /// The number of the newest sentence seen: the next list asks for what came after it.
    pub told_after: u64,
    /// The apps a helper may be given.
    pub apps: BTreeSet<AppName>,
    /// The card area.
    pub open: Open,
    /// The list of models on the open card.
    pub models: ModelList,
    /// The question about removing a helper.
    pub removal: Removal,
    /// A sentence about what was refused or could not be done, until the person does the next
    /// thing.
    pub notice: Option<String>,
}

impl HelpersModel {
    /// The helper whose card is open or on its way.
    pub fn open_id(&self) -> Option<&HelperId> {
        match &self.open {
            Open::Reading(id) => Some(id),
            Open::Card(card) => Some(&card.view.line.id),
            Open::Nothing | Open::New(_) => None,
        }
    }

    /// The draft the person is editing: a new card's, or the open card's.
    pub fn draft(&self) -> Option<&Draft> {
        match &self.open {
            Open::Card(card) => Some(&card.draft),
            Open::New(draft) => Some(draft),
            Open::Nothing | Open::Reading(_) => None,
        }
    }

    /// The line of the helper `id`.
    pub fn line(&self, id: &HelperId) -> Option<&HelperLine> {
        self.lines.iter().find(|line| line.id == *id)
    }
}
