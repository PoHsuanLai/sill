//! The accounts loop: accountd's signals, the watches on each account, the notifications
//! server's answers and the person's commands, stepping [`AccountsState`] and posting its notices.

use std::collections::HashMap;

use porter_dbus::ManagerProxy;
use sill_model::accounts::{
    AccountKey, AccountsCommand, AccountsEffect, AccountsEvent, AccountsLink, AccountsState,
    NeedsSignIn, shown_for, step,
};
use sill_model::notifications::{NotesState, NotificationsCommand};
use tokio::runtime::Handle;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};
use tokio::sync::watch;
use tokio::task::AbortHandle;
use zbus::{Connection, MessageStream};

use super::links::AccountsLinks;
use super::{actions, notice, wire};
use crate::accounts::AccountsService;
use crate::control::Control;
use crate::env::Bus;
use crate::error::ServiceError;
use crate::owner::{self, Ended};
use crate::service::ServiceIo;
use crate::stream::next;

/// Everything the loop owns.
struct Live {
    connection: Connection,
    rt: Handle,
    told: UnboundedSender<AccountsEvent>,
    tx: watch::Sender<AccountsState>,
    state: AccountsState,
    notes: watch::Receiver<NotesState>,
    notifications: Control<NotificationsCommand>,
    /// One watch per account that needs signing in again, ended when it is well.
    watches: HashMap<AccountKey, AbortHandle>,
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
    let connection = bus.connected().await?;
    let mut actions = actions::stream(&connection).await?;
    let (told, mut events) = unbounded_channel();
    let mut live = Live {
        connection: connection.clone(),
        rt: Handle::current(),
        told,
        tx,
        state: AccountsState::default(),
        notes: links.notes,
        notifications: links.notifications,
        watches: HashMap::new(),
    };
    loop {
        owner::appeared(&connection, wire::ACCOUNTS).await;
        tokio::select! {
            ended = live.session(&mut commands, &mut actions, &mut events) => match ended {
                Ended::Commands => return Ok(()),
                Ended::Unreadable => owner::changed(&connection, wire::ACCOUNTS).await,
            },
            () = owner::changed(&connection, wire::ACCOUNTS) => {}
        }
        live.lost();
    }
}

impl Live {
    /// One accountd lifetime: meet it, then follow its signals, the watches, the server's
    /// answers and the commands.
    async fn session(
        &mut self,
        commands: &mut UnboundedReceiver<AccountsCommand>,
        actions: &mut MessageStream,
        events: &mut UnboundedReceiver<AccountsEvent>,
    ) -> Ended {
        let Ok(manager) = ManagerProxy::new(&self.connection).await else {
            return Ended::Unreadable;
        };
        // Open before asking, so no signal falls between the list and the wait.
        let Ok(mut signals) = manager.receive_needs_reauth().await else {
            return Ended::Unreadable;
        };
        self.apply(AccountsEvent::Found);
        match wire::listed(&self.connection).await {
            Ok(list) => self.apply(AccountsEvent::Listed(list)),
            Err(error) => {
                eprintln!("sill: accounts: asking who needs signing in: {error}");
                return Ended::Unreadable;
            }
        }
        loop {
            tokio::select! {
                Some(signal) = next(&mut signals) => {
                    if let Ok(args) = signal.args() {
                        self.heard(AccountKey::new(args.account().as_str()));
                    }
                }
                Some(event) = events.recv() => self.apply(event),
                Some(Ok(message)) = next(actions) => {
                    if let Some(account) = actions::pressed(&self.connection, &message).await {
                        self.sign_in(&account);
                    }
                }
                command = commands.recv() => match command {
                    Some(AccountsCommand::SignInAgain(account)) => self.sign_in(&account),
                    None => return Ended::Commands,
                },
            }
        }
    }

    /// accountd says `account` needs signing in again: read its label, then tell the step.
    fn heard(&self, account: AccountKey) {
        let (connection, told) = (self.connection.clone(), self.told.clone());
        self.rt.spawn(async move {
            let label = wire::label_of(&connection, &account).await;
            let _ = told.send(AccountsEvent::NeedsSignIn(NeedsSignIn { account, label }));
        });
    }

    /// Ask accountd to sign `account` in again, when it is one that needs it.
    fn sign_in(&self, account: &AccountKey) {
        if !self.state.needing.iter().any(|n| n.account == *account) {
            return;
        }
        let (connection, account) = (self.connection.clone(), account.clone());
        self.rt.spawn(async move {
            if let Err(error) = wire::sign_in_again(&connection, &account).await {
                eprintln!("sill: accounts: signing in again: {error}");
            }
        });
    }

    /// accountd is gone: its watches end, and what the person was told stays until it is back.
    fn lost(&mut self) {
        for (_, watch) in self.watches.drain() {
            watch.abort();
        }
        self.apply(AccountsEvent::Gone);
    }

    /// Step the state by `event`, publish it, and carry out what the step asks.
    fn apply(&mut self, event: AccountsEvent) {
        let (next, effects) = step(std::mem::take(&mut self.state), event);
        self.tx.send_if_modified(|published| {
            let changed = *published != next;
            if changed {
                published.clone_from(&next);
            }
            changed
        });
        self.state = next;
        for effect in effects {
            self.carry_out(effect);
        }
        self.follow();
    }

    fn carry_out(&self, effect: AccountsEffect) {
        let command = match effect {
            AccountsEffect::Tell(needing) => {
                let replaces = shown_for(&self.notes.borrow(), &needing.account);
                Some(NotificationsCommand::Post(Box::new(notice::posting(
                    &needing, replaces,
                ))))
            }
            AccountsEffect::Withdraw(account) => {
                shown_for(&self.notes.borrow(), &account).map(NotificationsCommand::Dismiss)
            }
        };
        if let Some(command) = command {
            // A stopped notifications service has nobody to tell.
            let _ = self.notifications.send(command);
        }
    }

    /// Watch each account that needs signing in again while accountd is there, and no other.
    fn follow(&mut self) {
        let wanted: Vec<&AccountKey> = match self.state.link {
            AccountsLink::Present => self.state.needing.iter().map(|n| &n.account).collect(),
            AccountsLink::Absent => Vec::new(),
        };
        self.watches.retain(|account, watch| {
            let keep = wanted.contains(&account);
            if !keep {
                watch.abort();
            }
            keep
        });
        for account in wanted {
            if !self.watches.contains_key(account) {
                let task = self.rt.spawn(wire::watch(
                    self.connection.clone(),
                    account.clone(),
                    self.told.clone(),
                ));
                self.watches.insert(account.clone(), task.abort_handle());
            }
        }
    }
}
