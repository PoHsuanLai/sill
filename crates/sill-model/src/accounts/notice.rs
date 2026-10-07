//! Telling the person: the words, the one action, and finding the notice already shown.

use crate::notifications::{ActionKey, NotesState, NotificationId};

use super::model::{AccountKey, NeedsSignIn, SignInAsk};

/// The desktop entry the notices carry; it names them among the notifications held.
pub const NOTICE_ENTRY: &str = "sill-accounts";

/// What an action key says before the account it is for.
const KEY_PREFIX: &str = "sign-in-again:";

/// A notification's words and the label of its one button.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Notice {
    pub summary: String,
    pub body: String,
    pub button: String,
}

/// What the person is told about `needing`.
pub fn notice_text(needing: &NeedsSignIn) -> Notice {
    match needing.ask {
        SignInAsk::Again => Notice {
            summary: format!("{} needs you to sign in again", needing.label),
            body: "Apps cannot use this account until you do.".to_owned(),
            button: "Sign in again".to_owned(),
        },
        SignInAsk::Login => Notice {
            summary: format!("Sign in to {} to use it again", needing.label),
            body: "Apps cannot use this agent until you do.".to_owned(),
            button: "Sign in".to_owned(),
        },
    }
}

/// The action key of the button that signs `account` in again: it carries the account, so the
/// answer to the button needs no memory of what was posted.
pub fn sign_in_key(account: &AccountKey) -> ActionKey {
    ActionKey(format!("{KEY_PREFIX}{}", account.as_str()))
}

/// The account a button's action key signs in again; `None` for any other key.
pub fn account_of(key: &ActionKey) -> Option<AccountKey> {
    key.0
        .strip_prefix(KEY_PREFIX)
        .filter(|account| !account.is_empty())
        .map(AccountKey::new)
}

/// The sign-in notice for `account` already held, banner or history.
pub fn shown_for(notes: &NotesState, account: &AccountKey) -> Option<NotificationId> {
    let key = sign_in_key(account);
    let banners = notes.live.banners.iter().map(|banner| &banner.note);
    let kept = notes.history.groups.iter().flat_map(|group| &group.notes);
    banners
        .chain(kept)
        .filter(|note| note.app.desktop_entry.as_deref() == Some(NOTICE_ENTRY))
        .find(|note| note.actions.iter().any(|action| action.key == key))
        .map(|note| note.id)
}
