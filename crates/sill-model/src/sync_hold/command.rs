//! What can happen to the sync-hold service, what it can be told, and what it must then do.

use super::model::{Dataset, Hold};

/// What the service can be told.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyncHoldCommand {
    /// Join syncd again, to be told the holds now on.
    Rewatch,
}

/// One thing that happened.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyncHoldEvent {
    /// syncd says the dataset is held (again, with fresh counts).
    Held(Hold),
    /// The hold on the dataset ended, or syncd left.
    Ended(Dataset),
}

/// What the loop does as a result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyncHoldEffect {
    /// Show the notification for the hold, in place of one already shown for its dataset.
    Tell(Hold),
    /// Take the dataset's notification back.
    Withdraw(Dataset),
}
