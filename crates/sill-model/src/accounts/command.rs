//! What can happen to the accounts service, what it can be told, and what it must then do.

use super::model::{AccountKey, NeedsSignIn};

/// What the accounts service can be told.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccountsCommand {
    /// The person chose to sign in to this account again: accountd runs the sheet.
    SignInAgain(AccountKey),
}

/// One thing that happened.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccountsEvent {
    /// accountd is on the bus.
    Found,
    /// accountd left the bus, or was replaced.
    Gone,
    /// accountd's own list of the accounts needing sign-in, read on meeting it: replaces what was
    /// known.
    Listed(Vec<NeedsSignIn>),
    /// accountd said an account needs signing in again.
    NeedsSignIn(NeedsSignIn),
    /// The account is well again, or gone.
    SignedIn(AccountKey),
}

/// What the loop does as a result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccountsEffect {
    /// Tell the person, and watch the account for its recovery.
    Tell(NeedsSignIn),
    /// Take the notice back, and stop watching.
    Withdraw(AccountKey),
}
