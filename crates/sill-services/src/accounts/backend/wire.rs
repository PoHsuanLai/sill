//! accountd's side of the conversation: who needs signing in, an account's label and standing,
//! and the call that signs one in again.

use porter_client::{AccountView, AccountsError, LinkError, Malformed};
use porter_core::{AccountState, ReauthReason};
use porter_dbus::accounts::AccountProxy;
use porter_wire::ParentWindow;
use porter_wire::names::ACCOUNTS_BUS;
use sill_model::accounts::{AccountKey, AccountStanding, AccountsEvent, NeedsSignIn, SignInAsk};
use tokio::sync::mpsc::UnboundedSender;
use zbus::Connection;

use crate::account_label::{Accountd, key_of, ref_of};
use crate::stream::next;
use ds_core::word::Word;

/// accountd's well-known name.
pub(crate) const ACCOUNTS: &str = ACCOUNTS_BUS;

/// The accounts accountd says need signing in again, with their labels and the sign-in each asks
/// for.
pub(crate) async fn listed(accountd: &Accountd) -> Result<Vec<NeedsSignIn>, LinkError> {
    let list = accountd.needing_reauth().await?;
    let mut found = Vec::with_capacity(list.len());
    for needing in list {
        let account = key_of(&needing.account);
        let ask = ask_of(accountd, &account).await;
        found.push(NeedsSignIn {
            account,
            label: needing.label.0,
            ask,
        });
    }
    Ok(found)
}

/// Which sign-in the account asks for; a plain sign-in again when accountd will not say.
pub(crate) async fn ask_of(accountd: &Accountd, key: &AccountKey) -> SignInAsk {
    let view = view_of(accountd, key).await;
    let ask = view
        .as_ref()
        .map(|view| standing_of(view.state))
        .and_then(AccountStanding::ask)
        .unwrap_or_default();
    match ask {
        SignInAsk::Again => again_by_reason(view.as_ref()),
        SignInAsk::TestingExpired | SignInAsk::Login => ask,
    }
}

/// A plain sign-in again, or the one Google's testing-app limit asks for when accountd's
/// `ReauthReason` says `testing_app_expired`.
fn again_by_reason(view: Option<&AccountView>) -> SignInAsk {
    match view.and_then(|view| view.reauth_reason) {
        Some(ReauthReason::TestingAppExpired) => SignInAsk::TestingExpired,
        _ => SignInAsk::Again,
    }
}

/// The account as accountd shows it, or `None` when it will not say.
async fn view_of(accountd: &Accountd, key: &AccountKey) -> Option<AccountView> {
    accountd.view(&ref_of(key)?).await.ok()
}

/// How accountd says an account stands, as the shell words it.
fn standing_of(state: AccountState) -> AccountStanding {
    match state {
        AccountState::Ok => AccountStanding::Ok,
        AccountState::NeedsReauth => AccountStanding::NeedsReauth,
        AccountState::NeedsLogin => AccountStanding::NeedsLogin,
        AccountState::Offline => AccountStanding::Offline,
        AccountState::Limited => AccountStanding::Limited,
    }
}

/// Ask accountd to sign the account in again; the sign-in itself is drawn by sill's own sheet.
/// Returns once the sheet has been answered.
pub(crate) async fn sign_in_again(
    accountd: &Accountd,
    key: &AccountKey,
) -> Result<(), AccountsError> {
    let account = ref_of(key).ok_or(LinkError::Malformed(Malformed::Id))?;
    accountd
        .reauthenticate_shown(&account, &ParentWindow::Unparented)
        .await
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

/// accountd's object for the account `key`. The client has no stream of an account's `State`
/// changes, so the watch below reads them from the proxy.
async fn account(connection: &Connection, key: &AccountKey) -> zbus::Result<AccountProxy<'static>> {
    AccountProxy::builder(connection)
        .path(key.as_str().to_owned())?
        .build()
        .await
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
