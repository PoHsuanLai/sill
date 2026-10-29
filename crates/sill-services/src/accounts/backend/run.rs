//! The accounts loop: accountd's changes, the notifications server's answers and the person's
//! commands, stepping [`AccountsState`] and posting its notices.

use porter_client::{AccountChange, AccountsError};
use porter_core::AccountState;
use porter_wire::Refusal;
use sill_model::accounts::{
    AccountKey, AccountsCommand, AccountsEffect, AccountsEvent, AccountsState, NeedsSignIn,
    account_of, shown_for, step,
};
use sill_model::notifications::{NotificationsCommand, Said};
use tokio::runtime::Handle;
use tokio::sync::broadcast;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};

use super::links::AccountsLinks;
use super::retry::{self, Retry, Wait};
use super::{notice, wire};
use crate::account_label::{Accountd, accountd, key_of, label_of};
use crate::accounts::AccountsService;
use crate::env::Bus;
use crate::error::ServiceError;
use crate::machine::Machine;
use crate::notices::Board;
use crate::owner::{self, Ended};
use crate::service::ServiceIo;

/// Everything the loop owns.
struct Live {
    accounts: Accountd,
    rt: Handle,
    told: UnboundedSender<AccountsEvent>,
    machine: Machine<AccountsState, AccountsEvent, AccountsEffect>,
    /// The sign-in notices, one per account.
    board: Board<AccountKey>,
}

/// Runs until the commands close. Without accountd the state stays at its default; accountd
/// starting late, leaving or restarting is followed through `NameOwnerChanged`.
pub(crate) async fn run(
    bus: Bus,
    links: AccountsLinks,
    io: ServiceIo<AccountsService>,
) -> Result<(), ServiceError> {
    let ServiceIo {
        state: tx,
        mut commands,
    } = io;
    // Listening before the bus is reached, so no answer falls between.
    let mut answers = links.answers.hear();
    let connection = bus.connect_waiting().await?;
    let (told, mut events) = unbounded_channel();
    let mut live = Live {
        accounts: accountd(&connection),
        rt: Handle::current(),
        told,
        machine: Machine::new(tx, step),
        board: Board::new(links.notifications, links.notes, shown_for),
    };
    let owner = owner::Owner::of(&connection, wire::ACCOUNTS);
    while owner
        .session(live.lifetime(&mut commands, &mut answers, &mut events))
        .await
        == owner::Next::Again
    {
        live.lost();
    }
    Ok(())
}

/// How a session ended: as every service's does, or with accountd refusing the first call.
enum Met {
    Ended(Ended),
    Refused,
}

impl Live {
    /// One accountd owner's lifetime, with the meeting tried again while accountd refuses it: a
    /// short, growing wait, and then only the name changing hands (which ends this future and
    /// starts the tries over). Nothing is shown as found while the tries wait.
    async fn lifetime(
        &mut self,
        commands: &mut UnboundedReceiver<AccountsCommand>,
        answers: &mut broadcast::Receiver<Said>,
        events: &mut UnboundedReceiver<AccountsEvent>,
    ) -> Ended {
        let mut retry = Retry::default();
        loop {
            match self.session(commands, answers, events).await {
                Met::Ended(ended) => return ended,
                Met::Refused => {
                    self.lost();
                    match retry.refused() {
                        (Wait::Then(after), spent) => {
                            tokio::time::sleep(after).await;
                            retry = spent;
                        }
                        (Wait::OnNameChange, _) => return Ended::Unreadable,
                    }
                }
            }
        }
    }

    /// One accountd lifetime: meet it, then follow its changes, the server's answers and the
    /// commands.
    async fn session(
        &mut self,
        commands: &mut UnboundedReceiver<AccountsCommand>,
        answers: &mut broadcast::Receiver<Said>,
        events: &mut UnboundedReceiver<AccountsEvent>,
    ) -> Met {
        // Open before asking, so no signal falls between the list and the wait. Opening is also
        // the first call accountd answers, so a refusal of it is the one to try again.
        let mut changes = match self.accounts.watch().await {
            Ok(changes) => changes,
            Err(error) if retry::is_refusal(&error) => return Met::Refused,
            Err(_) => return Met::Ended(Ended::Unreadable),
        };
        self.apply(AccountsEvent::Found);
        match wire::listed(&self.accounts).await {
            Ok(list) => self.apply(AccountsEvent::Listed(list)),
            Err(error) if retry::is_refusal(&error) => return Met::Refused,
            Err(error) => {
                eprintln!("sill: accounts: asking who needs signing in: {error}");
                return Met::Ended(Ended::Unreadable);
            }
        }
        loop {
            tokio::select! {
                Some(change) = changes.next() => match change {
                    Ok(AccountChange::State(account, state)) => self.heard(key_of(&account), state),
                    Ok(AccountChange::Removed(account)) => {
                        self.apply(AccountsEvent::SignedIn(key_of(&account)));
                    }
                    _ => {}
                },
                Some(event) = events.recv() => self.apply(event),
                Ok(said) = answers.recv() => self.heard_server(said),
                Ok(()) = self.board.changed() => self.learn(),
                command = commands.recv() => match command {
                    Some(AccountsCommand::SignInAgain(account)) => self.sign_in(&account),
                    None => return Met::Ended(Ended::Commands),
                },
            }
        }
    }

    /// Keep the id of each notice the notes now hold for an account that needs the person.
    fn learn(&mut self) {
        let accounts: Vec<AccountKey> = self
            .machine
            .state()
            .needing
            .iter()
            .map(|n| n.account.clone())
            .collect();
        self.board.learn(&accounts);
    }

    /// The notifications server's word: a press of a notice sill posted signs the account in
    /// again; a close ends the notice's claim.
    fn heard_server(&mut self, said: Said) {
        // The notes may already have let go of a notice just pressed: what they held is learnt
        // before the press is weighed.
        self.learn();
        match said {
            Said::Pressed { id, key } => {
                if let Some(account) = self.board.answers(id, account_of(&key), |account| account) {
                    self.sign_in(&account);
                }
            }
            Said::Closed { id, .. } => self.board.closed(id),
        }
    }

    /// accountd says `account` is in `state` now. One that needs the person: read its label, then
    /// tell the step; any other state means it no longer needs signing in.
    fn heard(&mut self, account: AccountKey, state: AccountState) {
        if !wire::standing_of(state).needs_sign_in() {
            self.apply(AccountsEvent::SignedIn(account));
            return;
        }
        let (accounts, told) = (self.accounts.clone(), self.told.clone());
        self.rt.spawn(async move {
            let label = label_of(&accounts, &account).await;
            let ask = wire::ask_of(&accounts, &account).await;
            let _ = told.send(AccountsEvent::NeedsSignIn(NeedsSignIn {
                account,
                label,
                ask,
            }));
        });
    }

    /// Ask accountd to sign `account` in again, when it is one that needs it.
    fn sign_in(&self, account: &AccountKey) {
        if !self
            .machine
            .state()
            .needing
            .iter()
            .any(|n| n.account == *account)
        {
            return;
        }
        let (accounts, account) = (self.accounts.clone(), account.clone());
        let notifications = self.board.notifications();
        self.rt.spawn(async move {
            match wire::sign_in_again(&accounts, &account).await {
                Ok(()) => {}
                Err(AccountsError::Refused(Refusal::NoLauncher)) => {
                    let label = label_of(&accounts, &account).await;
                    let post = notice::no_launcher_posting(&label);
                    let _ = notifications.send(NotificationsCommand::Post(Box::new(post)));
                }
                Err(error) => eprintln!("sill: accounts: signing in again: {error}"),
            }
        });
    }

    /// accountd is gone: what the person was told stays until it is back.
    fn lost(&mut self) {
        self.apply(AccountsEvent::Gone);
    }

    /// Step the state by `event`, publish it, and carry out what the step asks.
    fn apply(&mut self, event: AccountsEvent) {
        for effect in self.machine.feed(event) {
            self.carry_out(effect);
        }
    }

    fn carry_out(&mut self, effect: AccountsEffect) {
        match effect {
            AccountsEffect::Tell(needing) => self.board.post(&needing.account, |replaces| {
                notice::posting(&needing, replaces)
            }),
            AccountsEffect::Withdraw(account) => self.board.withdraw(&account),
        }
    }
}
