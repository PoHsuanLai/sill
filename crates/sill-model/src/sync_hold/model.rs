//! The state types (data only).

use crate::launch::SettingsAccount;

/// syncd's name for a dataset, `<account>/<dataset>`: opaque here, and the only thing a signal
/// and the notification's action both say it by.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Dataset(String);

impl Dataset {
    /// The name syncd gave.
    pub fn new(name: impl Into<String>) -> Dataset {
        Dataset(name.into())
    }

    /// The name as syncd spells it.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A dataset whose replica lost files that syncd will not remove here until the person decides.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hold {
    pub dataset: Dataset,
    /// What the person calls the account (`Work files`).
    pub account_name: String,
    /// How many files would be removed here.
    pub discard: u64,
    /// How many files the dataset holds.
    pub held: u64,
    /// The account's object path, where Settings opens.
    pub account: SettingsAccount,
}

/// The holds now on, in the order they were told.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SyncHoldState {
    pub holds: Vec<Hold>,
}
