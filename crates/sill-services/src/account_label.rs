//! accountd's accounts by their keys: the handle that reads them and the label the notices that
//! name an account call it by. The accounts service and the sync-hold service both name accounts,
//! so neither reaches into the other for this.

use porter_client::{AccountRef, Accounts, Porter};
use porter_client_dbus::Bus;
use porter_wire::names::ACCOUNTS_PATH;
use sill_model::accounts::AccountKey;
use zbus::Connection;

/// What a notice calls an account whose label cannot be read.
const UNNAMED: &str = "An account";

/// accountd's accounts, over the shell's own bus connection.
pub(crate) type Accountd = Accounts<Bus>;

/// The handle on accountd over `connection`.
pub(crate) fn accountd(connection: &Connection) -> Accountd {
    Porter::over(Bus::over(connection.clone())).accounts()
}

/// The account `key` (accountd's object path for it) as the client names it; `None` for a key
/// that is not one.
pub(crate) fn ref_of(key: &AccountKey) -> Option<AccountRef> {
    AccountRef::of_object_path(key.as_str())
}

/// The key of `account`: accountd's object path for it.
pub(crate) fn key_of(account: &AccountRef) -> AccountKey {
    AccountKey::new(format!("{ACCOUNTS_PATH}/account/{}", account.segment()))
}

/// The account's label, or a plain stand-in when accountd will not say.
pub(crate) async fn label_of(accountd: &Accountd, key: &AccountKey) -> String {
    match ref_of(key) {
        Some(account) => accountd.view(&account).await.ok().map(|view| view.label.0),
        None => None,
    }
    .filter(|label| !label.is_empty())
    .unwrap_or_else(|| UNNAMED.to_owned())
}
