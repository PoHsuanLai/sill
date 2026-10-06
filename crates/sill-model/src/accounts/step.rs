//! The pure transition: what the accounts service knows after an event, and what it must do.

use super::command::{AccountsEffect, AccountsEvent};
use super::model::{AccountsLink, AccountsState};

/// `state` after `event`, with the effects it calls for. An account already known is not told
/// twice; accountd leaving changes nothing the person was told, and the list it gives on its
/// return is compared with what was.
pub fn step(state: AccountsState, event: AccountsEvent) -> (AccountsState, Vec<AccountsEffect>) {
    let AccountsState { link, mut needing } = state;
    let mut effects = Vec::new();
    let link = match event {
        AccountsEvent::Found => AccountsLink::Present,
        AccountsEvent::Gone => AccountsLink::Absent,
        AccountsEvent::Listed(listed) => {
            effects.extend(
                needing
                    .iter()
                    .filter(|known| !listed.iter().any(|now| now.account == known.account))
                    .map(|gone| AccountsEffect::Withdraw(gone.account.clone())),
            );
            effects.extend(
                listed
                    .iter()
                    .filter(|now| !needing.iter().any(|known| known.account == now.account))
                    .cloned()
                    .map(AccountsEffect::Tell),
            );
            needing = listed;
            link
        }
        AccountsEvent::NeedsSignIn(found) => {
            if !needing.iter().any(|known| known.account == found.account) {
                needing.push(found.clone());
                effects.push(AccountsEffect::Tell(found));
            }
            link
        }
        AccountsEvent::SignedIn(account) => {
            if needing.iter().any(|known| known.account == account) {
                needing.retain(|known| known.account != account);
                effects.push(AccountsEffect::Withdraw(account));
            }
            link
        }
    };
    (AccountsState { link, needing }, effects)
}
