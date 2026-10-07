//! The accounts loop: accountd's signals, the watches on each account, the notifications
//! server's answers and the person's commands, stepping [`AccountsState`] and posting its notices.

use std::collections::HashMap;

use porter_dbus::ManagerProxy;
use sill_model::accounts::{
    AccountKey, AccountsCommand, AccountsEffect, AccountsEvent, AccountsLink, AccountsState,
    NeedsSignIn, Posted, shown_for, step,
};
use sill_model::notifications::{NotesState, NotificationsCommand};
use tokio::runtime::Handle;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};
use tokio::sync::watch;
use tokio::task::AbortHandle;
use zbus::{Connection, MessageStream};

use super::actions::Said;
use super::links::AccountsLinks;
use super::retry::{self, Retry, Wait};
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
    /// The ids of the notices sill posted, the only ones whose buttons answer.
    posted: Posted,
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
        posted: Posted::default(),
        watches: HashMap::new(),
    };
    let mut retry = Retry::default();
    loop {
        owner::appeared(&connection, wire::ACCOUNTS).await;
        tokio::select! {
            met = live.session(&mut commands, &mut actions, &mut events) => match met {
                Met::Ended(Ended::Commands) => return Ok(()),
                Met::Ended(Ended::Unreadable) => {
                    owner::changed(&connection, wire::ACCOUNTS).await;
                    retry = Retry::default();
                }
                Met::Refused => {
                    // Not met yet: nothing is shown as found while the tries wait.
                    live.lost();
                    retry = after_refusal(&connection, retry).await;
                }
            },
            () = owner::changed(&connection, wire::ACCOUNTS) => retry = Retry::default(),
        }
        live.lost();
    }
}

/// How a session ended: as every service's does, or with accountd refusing the first call.
enum Met {
    Ended(Ended),
    Refused,
}

/// Wait out a refusal: a short backoff, then the name changing hands. A name change at any point
/// starts the tries over.
async fn after_refusal(connection: &Connection, retry: Retry) -> Retry {
    let (wait, spent) = retry.refused();
    let changed = owner::changed(connection, wire::ACCOUNTS);
    match wait {
        Wait::Then(after) => tokio::select! {
            () = tokio::time::sleep(after) => spent,
            () = changed => Retry::default(),
        },
        Wait::OnNameChange => {
            changed.await;
            Retry::default()
        }
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
    ) -> Met {
        let Ok(manager) = ManagerProxy::new(&self.connection).await else {
            return Met::Ended(Ended::Unreadable);
        };
        // Open before asking, so no signal falls between the list and the wait.
        let Ok(mut signals) = manager.receive_needs_reauth().await else {
            return Met::Ended(Ended::Unreadable);
        };
        self.apply(AccountsEvent::Found);
        match wire::listed(&self.connection).await {
            Ok(list) => self.apply(AccountsEvent::Listed(list)),
            Err(error) if retry::is_refusal(&error) => return Met::Refused,
            Err(error) => {
                eprintln!("sill: accounts: asking who needs signing in: {error}");
                return Met::Ended(Ended::Unreadable);
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
                    if let Some(said) = actions::said(&self.connection, &message).await {
                        self.heard_server(said);
                    }
                }
                Ok(()) = self.notes.changed() => self.learn(),
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
            .state
            .needing
            .iter()
            .map(|n| n.account.clone())
            .collect();
        self.posted.learn(&self.notes.borrow(), &accounts);
    }

    /// The notifications server's word: a press of a notice sill posted signs the account in
    /// again; a close ends the notice's claim.
    fn heard_server(&mut self, said: Said) {
        // The notes may already have let go of a notice just pressed: what they held is learnt
        // before the press is weighed.
        self.learn();
        match said {
            Said::Pressed { id, key } => {
                if let Some(account) = self.posted.answers(id, &key) {
                    self.sign_in(&account);
                }
            }
            Said::Closed(id) => self.posted.closed(id),
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

    fn carry_out(&mut self, effect: AccountsEffect) {
        let command = match effect {
            AccountsEffect::Tell(needing) => {
                self.posted.forget(&needing.account);
                let replaces = shown_for(&self.notes.borrow(), &needing.account);
                Some(NotificationsCommand::Post(Box::new(notice::posting(
                    &needing, replaces,
                ))))
            }
            AccountsEffect::Withdraw(account) => {
                self.posted.forget(&account);
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
