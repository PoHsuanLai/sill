//! The state types (data only).

use ds_core::word::Word;

/// How accountd names an account to its clients: opaque here, and the only thing a signal, a
/// reply and the notification's action all say it by.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct AccountKey(String);

impl AccountKey {
    /// The key accountd gave.
    pub fn new(key: impl Into<String>) -> AccountKey {
        AccountKey(key.into())
    }

    /// The key as accountd spells it.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// An account that stopped working until the person signs in to it again.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NeedsSignIn {
    pub account: AccountKey,
    /// What the person calls it (`Work mail`).
    pub label: String,
    /// Which sign-in it needs.
    pub ask: SignInAsk,
}

/// Which sign-in an account needs, for the words that tell the person.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SignInAsk {
    /// The sign-in held no longer works: sign in again.
    #[default]
    Again,
    /// Google signed the account out because the app is still in testing (every seven days until
    /// it is verified): sign in again, and the words say why.
    TestingExpired,
    /// A coding agent that has to be signed in, inside the agent: sign in to use it again.
    Login,
}

/// Whether accountd is on the bus. Its absence is a state: a machine without it has nothing to
/// sign in to, and a notice already shown stays until the account is seen to be well again.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AccountsLink {
    /// Not on the bus, or not yet heard from.
    #[default]
    Absent,
    /// accountd answers.
    Present,
}

/// What is known of the accounts, published by the service.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AccountsState {
    pub link: AccountsLink,
    /// The accounts that need signing in again, in the order they were found.
    pub needing: Vec<NeedsSignIn>,
}

/// How accountd says an account stands, as its `State` property spells it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Word)]
#[word(case = snake)]
pub enum AccountStanding {
    /// Works.
    Ok,
    /// The sign-in no longer holds: the person has to sign in again.
    NeedsReauth,
    /// A coding agent that must be signed in (inside the agent) before it can be used.
    NeedsLogin,
    /// The service cannot be reached; it will be tried again.
    Offline,
    /// Works with a restriction.
    Limited,
}

impl AccountStanding {
    /// Whether only the person can put it right.
    pub fn needs_sign_in(self) -> bool {
        matches!(
            self,
            AccountStanding::NeedsReauth | AccountStanding::NeedsLogin
        )
    }

    /// Which sign-in it asks for, when it asks for one.
    pub fn ask(self) -> Option<SignInAsk> {
        match self {
            AccountStanding::NeedsReauth => Some(SignInAsk::Again),
            AccountStanding::NeedsLogin => Some(SignInAsk::Login),
            AccountStanding::Ok | AccountStanding::Offline | AccountStanding::Limited => None,
        }
    }
}
