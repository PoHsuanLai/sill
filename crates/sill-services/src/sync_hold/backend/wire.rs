//! syncd's `NeedsConfirmation` signal, as a hold or the end of one.

use porter_dbus::Details;
use sill_launcher::settings_target::{AccountRef, SettingsTarget};
use sill_model::accounts::AccountKey;
use sill_model::sync_hold::{Dataset, Hold};
use zbus::zvariant::OwnedValue;

/// syncd's well-known name.
pub(crate) const SYNC: &str = ferry_bus::SYNC_BUS;

/// What a `NeedsConfirmation` signal says about a dataset.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Heard {
    /// The dataset is held: how many files would go, how many it holds, and the account's path.
    Held {
        dataset: Dataset,
        discard: u64,
        held: u64,
        account: AccountKey,
    },
    /// The hold ended (an empty dict).
    Ended(Dataset),
}

/// The signal's meaning. A dict that is not empty but lacks a count or the account is not a hold
/// the person can act on, so it says nothing.
pub(crate) fn heard(dataset: &str, details: &Details) -> Option<Heard> {
    let dataset = Dataset::new(dataset);
    if details.is_empty() {
        return Some(Heard::Ended(dataset));
    }
    let count = |key: &str| -> Option<u64> { details.get(key).and_then(|v| u64::try_from(v).ok()) };
    let account = details
        .get("account")
        .and_then(|value: &OwnedValue| String::try_from(value.clone()).ok())
        .filter(|path| !path.is_empty())?;
    Some(Heard::Held {
        dataset,
        discard: count("discard")?,
        held: count("held")?,
        account: AccountKey::new(account),
    })
}

impl Heard {
    /// The hold, once the account's name is known.
    pub(crate) fn hold(self, account_name: String) -> Option<Hold> {
        match self {
            Heard::Held {
                dataset,
                discard,
                held,
                account,
            } => Some(Hold {
                dataset,
                account_name,
                discard,
                held,
                account: SettingsTarget::Account(AccountRef(account.as_str().to_owned())),
            }),
            Heard::Ended(_) => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn value<'a>(v: impl Into<zbus::zvariant::Value<'a>>) -> OwnedValue {
        OwnedValue::try_from(v.into()).unwrap_or_else(|e| panic!("{e}"))
    }

    #[test]
    fn a_dict_with_counts_and_an_account_is_a_hold_and_an_empty_one_is_its_end() {
        let mut held = Details::new();
        held.insert("discard".to_owned(), value(30_u64));
        held.insert("held".to_owned(), value(40_u64));
        held.insert(
            "account".to_owned(),
            value("/org/quire/Accounts1/account/work"),
        );
        assert_eq!(
            heard("work/files", &held),
            Some(Heard::Held {
                dataset: Dataset::new("work/files"),
                discard: 30,
                held: 40,
                account: AccountKey::new("/org/quire/Accounts1/account/work"),
            })
        );
        assert_eq!(
            heard("work/files", &Details::new()),
            Some(Heard::Ended(Dataset::new("work/files")))
        );
        held.remove("account");
        assert_eq!(heard("work/files", &held), None);
    }
}
