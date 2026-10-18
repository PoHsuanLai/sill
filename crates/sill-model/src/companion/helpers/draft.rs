//! The helper card the person is filling in: raw words and choices, checked once when they save.
//! A card that already exists is read back into a draft; what it may use arrives as plain lines,
//! so [`reach_of_lines`] turns them into the apps and the one limit the form asks for.

use std::collections::{BTreeSet, HashSet};

use companion_wire::{HelperCardView, HelperForm};
use docket_core::helper::{Ceiling, HelperKind, HelperModel, HelperName, HelperTextFault, Role};
use porter_core::Count;
use prov::{AppName, SpaceId};

/// The most steps one job takes when the person has not said.
pub(super) const DEFAULT_STEPS: Count = Count(12);

/// What a helper may use: the apps the person ticked, and how far it may go in all of them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reach {
    /// Apps and a limit the person can read and change.
    Chosen {
        /// The apps it may use.
        apps: BTreeSet<AppName>,
        /// How far it may go in them.
        ceiling: Ceiling,
    },
    /// The card's lines were not ones the form can read back, so the form cannot save it until
    /// the person chooses afresh.
    Unreadable,
}

/// Why a draft cannot be saved yet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum SaveFault {
    /// No name, or only spaces.
    #[error("Give it a name.")]
    NoName,
    /// A name that is too long, or has a character that is not text.
    #[error("That name is too long or has a character it cannot keep.")]
    BadName,
    /// No words for what it is for.
    #[error("Say what it is for.")]
    NoRole,
    /// What it is for is too long, or has a character that is not text.
    #[error("What it is for is too long or has a character it cannot keep.")]
    BadRole,
    /// No app ticked.
    #[error("Choose at least one app for it to use.")]
    NoApps,
    /// The card's apps could not be read back.
    #[error("Choose the apps it may use again to save this card.")]
    ReachUnreadable,
}

/// A helper card being made or changed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Draft {
    /// What they call it.
    pub name: String,
    /// What it is for.
    pub role: String,
    /// What it may use.
    pub reach: Reach,
    /// Its model; `Unset` until the person chooses.
    pub model: HelperModel,
    /// The most steps one job takes.
    pub steps: Count,
    /// Its Space.
    pub space: SpaceId,
    /// How it works; a card read back keeps the app it runs.
    pub kind: HelperKind,
}

impl Draft {
    /// A new card: nothing filled in, the narrowest limit, and no model.
    pub fn blank(space: SpaceId) -> Draft {
        Draft {
            name: String::new(),
            role: String::new(),
            reach: Reach::Chosen {
                apps: BTreeSet::new(),
                ceiling: Ceiling::Look,
            },
            model: HelperModel::Unset,
            steps: DEFAULT_STEPS,
            space,
            kind: HelperKind::Native,
        }
    }

    /// The draft that says what `view` says.
    pub fn of(view: &HelperCardView) -> Draft {
        Draft {
            name: view.line.name.as_str().to_owned(),
            role: view.role.clone(),
            reach: reach_of_lines(&view.reach),
            model: view.model.clone(),
            steps: view.steps,
            space: view.line.space.clone(),
            kind: view.kind.clone(),
        }
    }

    /// The form to send, or the first thing missing from it: a name, what it is for, an app.
    pub fn form(&self) -> Result<HelperForm, SaveFault> {
        let name = HelperName::parse(&self.name).map_err(|fault| match fault {
            HelperTextFault::Empty => SaveFault::NoName,
            _ => SaveFault::BadName,
        })?;
        let role = Role::parse(&self.role).map_err(|fault| match fault {
            HelperTextFault::Empty => SaveFault::NoRole,
            _ => SaveFault::BadRole,
        })?;
        let (apps, ceiling) = match &self.reach {
            Reach::Chosen { apps, ceiling } if !apps.is_empty() => (apps, *ceiling),
            Reach::Chosen { .. } => return Err(SaveFault::NoApps),
            Reach::Unreadable => return Err(SaveFault::ReachUnreadable),
        };
        Ok(HelperForm {
            name,
            role,
            space: self.space.clone(),
            model: self.model.clone(),
            apps: apps.iter().cloned().collect(),
            ceiling,
            steps: self.steps,
            kind: self.kind.clone(),
        })
    }
}

/// How a card's lines word each limit, as docket words them (`reach_lines`). A line in any other
/// words is not one the form can read back.
const LIMIT_PHRASES: [(&str, Ceiling); 3] = [
    ("look at things", Ceiling::Look),
    (
        "look at things and change them (changes can be undone)",
        Ceiling::Change,
    ),
    (
        "look at things, change them and send things out",
        Ceiling::Send,
    ),
];

/// The apps and the one limit that card lines such as `org.quire.Notes: look at things` say. A
/// line of any other form, or lines that disagree about the limit, are [`Reach::Unreadable`].
pub fn reach_of_lines(lines: &[String]) -> Reach {
    let mut apps = BTreeSet::new();
    let mut ceilings = HashSet::new();
    for line in lines {
        let Some((app, limit)) = line.split_once(": ") else {
            return Reach::Unreadable;
        };
        let ceiling = LIMIT_PHRASES
            .iter()
            .find(|(phrase, _)| *phrase == limit)
            .map(|(_, ceiling)| *ceiling);
        let (Ok(app), Some(ceiling)) = (AppName::parse(app), ceiling) else {
            return Reach::Unreadable;
        };
        apps.insert(app);
        ceilings.insert(ceiling);
    }
    let mut limits = ceilings.into_iter();
    match (limits.next(), limits.next()) {
        (None, _) => Reach::Chosen {
            apps,
            ceiling: Ceiling::Look,
        },
        (Some(ceiling), None) => Reach::Chosen { apps, ceiling },
        (Some(_), Some(_)) => Reach::Unreadable,
    }
}
