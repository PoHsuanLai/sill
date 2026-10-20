//! An account of accountd's, by its key: the proxy that reads it and the label the notices that
//! name an account call it by. The accounts service and the sync-hold service both name accounts,
//! so neither reaches into the other for this.

use porter_dbus::AccountProxy;
use sill_model::accounts::AccountKey;
use zbus::Connection;

/// What a notice calls an account whose label cannot be read.
const UNNAMED: &str = "An account";

/// accountd's object for the account `key`.
pub(crate) async fn account(
    connection: &Connection,
    key: &AccountKey,
) -> zbus::Result<AccountProxy<'static>> {
    AccountProxy::builder(connection)
        .path(key.as_str().to_owned())?
        .build()
        .await
}

/// The account's label, or a plain stand-in when accountd will not say.
pub(crate) async fn label_of(connection: &Connection, key: &AccountKey) -> String {
    match account(connection, key).await {
        Ok(proxy) => proxy.label().await.ok(),
        Err(_) => None,
    }
    .filter(|label| !label.is_empty())
    .unwrap_or_else(|| UNNAMED.to_owned())
}
