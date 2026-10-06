mod notice;
mod standing;
mod step;

use super::{AccountKey, NeedsSignIn};

/// An account that needs signing in again.
fn needing(key: &str, label: &str) -> NeedsSignIn {
    NeedsSignIn {
        account: AccountKey::new(key),
        label: label.to_owned(),
    }
}
