//! accountd's side of the conversation: who needs signing in, an account's label and standing,
//! and the call that signs one in again.

use porter_dbus::{ACCOUNTS_BUS, AccountProxy, Details};
use sill_model::accounts::{AccountKey, AccountStanding, AccountsEvent, NeedsSignIn, SignInAsk};
use tokio::sync::mpsc::UnboundedSender;
use zbus::Connection;
use zbus::zvariant::OwnedObjectPath;

use crate::stream::next;
use ds_core::word::Word;

/// accountd's well-known name.
pub(crate) const ACCOUNTS: &str = ACCOUNTS_BUS;

/// accountd's `ReauthReason` word for Google's seven-day sign-out of an unverified app.
const TESTING_EXPIRED: &str = "testing_app_expired";

/// What the notice calls an account whose label cannot be read.
const UNNAMED: &str = "An account";

/// The accounts that need signing in again now, with their labels, as accountd lists them for the
/// shell; asking is also how accountd learns this connection, which it then sends `NeedsReauth`
/// to. An accountd that has no such call lists nothing.
#[zbus::proxy(
    interface = "org.quire.Accounts1.Manager",
    default_service = "org.quire.Accounts1",
    default_path = "/org/quire/Accounts1"
)]
pub(crate) trait ManagerSignIn {
    fn needing_reauth(&self) -> zbus::Result<Vec<(OwnedObjectPath, String)>>;
}

/// The accounts accountd says need signing in again.
pub(crate) async fn listed(connection: &Connection) -> zbus::Result<Vec<NeedsSignIn>> {
    let manager = ManagerSignInProxy::new(connection).await?;
    match manager.needing_reauth().await {
        Ok(list) => {
            let mut found = Vec::with_capacity(list.len());
            for (path, label) in list {
                let account = AccountKey::new(path.as_str());
                let ask = ask_of(connection, &account).await;
                found.push(NeedsSignIn {
                    account,
                    label,
                    ask,
                });
            }
            Ok(found)
        }
        Err(zbus::Error::MethodError(name, ..))
            if name.as_str() == "org.freedesktop.DBus.Error.UnknownMethod" =>
        {
            Ok(Vec::new())
        }
        Err(error) => Err(error),
    }
}

async fn account(connection: &Connection, key: &AccountKey) -> zbus::Result<AccountProxy<'static>> {
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

/// Which sign-in the account asks for; a plain sign-in again when accountd will not say.
pub(crate) async fn ask_of(connection: &Connection, key: &AccountKey) -> SignInAsk {
    let standing = match account(connection, key).await {
        Ok(proxy) => proxy.state().await.ok(),
        Err(_) => None,
    };
    let ask = standing
        .and_then(|word| AccountStanding::parse(&word))
        .and_then(AccountStanding::ask)
        .unwrap_or_default();
    match ask {
        SignInAsk::Again => again_by_reason(connection, key).await,
        SignInAsk::TestingExpired | SignInAsk::Login => ask,
    }
}

/// A plain sign-in again, or the one Google's testing-app limit asks for when accountd's
/// `ReauthReason` says `testing_app_expired`.
async fn again_by_reason(connection: &Connection, key: &AccountKey) -> SignInAsk {
    let reason = match account(connection, key).await {
        Ok(proxy) => proxy.reauth_reason().await.ok(),
        Err(_) => None,
    };
    match reason.as_deref() {
        Some(TESTING_EXPIRED) => SignInAsk::TestingExpired,
        _ => SignInAsk::Again,
    }
}

/// Ask accountd to sign the account in again; the sign-in itself is drawn by sill's own sheet.
pub(crate) async fn sign_in_again(connection: &Connection, key: &AccountKey) -> zbus::Result<()> {
    account(connection, key)
        .await?
        .reauthenticate("", &Details::new())
        .await
        .map(drop)
}

/// Whether the account has stopped needing the person: `Some(true)`, still needs them:
/// `Some(false)`, and `None` when accountd could not say.
fn recovered(standing: zbus::Result<String>) -> Option<bool> {
    match standing {
        Ok(word) => AccountStanding::parse(&word).map(|standing| !standing.needs_sign_in()),
        Err(zbus::Error::FDO(error)) if matches!(*error, zbus::fdo::Error::UnknownObject(_)) => {
            Some(true)
        }
        Err(_) => None,
    }
}

/// Follow the account's `State` and say once it no longer needs signing in (or no longer
/// exists). Ends silently when accountd cannot be followed; the next session starts it over.
pub(crate) async fn watch(
    connection: Connection,
    key: AccountKey,
    told: UnboundedSender<AccountsEvent>,
) {
    let Ok(proxy) = account(&connection, &key).await else {
        return;
    };
    // Open first, so a change between the read and the wait is not lost.
    let mut changes = proxy.receive_state_changed().await;
    loop {
        if recovered(proxy.state().await) == Some(true) {
            let _ = told.send(AccountsEvent::SignedIn(key));
            return;
        }
        if next(&mut changes).await.is_none() {
            return;
        }
    }
}
