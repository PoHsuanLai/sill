//! The sync-hold loop: syncd's `NeedsConfirmation` signals, the notifications server's answers
//! and the commands, stepping [`SyncHoldState`] and posting its notices.

use porter_dbus::SyncProxy;
use sill_model::launch::{LaunchCommand, SettingsOpen};
use sill_model::notifications::{NotesState, NotificationsCommand};
use sill_model::sync_hold::{
    Dataset, Posted, SyncHoldCommand, SyncHoldEffect, SyncHoldEvent, SyncHoldState, shown_for, step,
};
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::sync::watch;
use zbus::{Connection, MessageStream};

use super::links::SyncHoldLinks;
use super::notice;
use super::wire::{self, Heard};
use crate::accounts::{actions, wire as accounts_wire};
use crate::control::Control;
use crate::env::Bus;
use crate::error::ServiceError;
use crate::owner::{self, Ended};
use crate::service::ServiceIo;
use crate::stream::next;
use crate::sync_hold::SyncHoldService;

/// Everything the loop owns.
struct Live {
    connection: Connection,
    tx: watch::Sender<SyncHoldState>,
    state: SyncHoldState,
    notes: watch::Receiver<NotesState>,
    notifications: Control<NotificationsCommand>,
    launch: Control<LaunchCommand>,
    /// The ids of the notices sill posted, the only ones whose buttons answer.
    posted: Posted,
}

/// Runs until the commands close. Without syncd the state stays empty; syncd starting late,
/// leaving or restarting is followed through `NameOwnerChanged`, and each new owner is joined
/// again with `Watch`.
pub(crate) async fn run(
    bus: Bus,
    links: SyncHoldLinks,
    io: ServiceIo<SyncHoldService>,
) -> Result<(), ServiceError> {
    let ServiceIo {
        state: tx,
        mut commands,
    } = io;
    let connection = crate::supervise::connect_when_up(&bus).await?;
    let mut answers = actions::stream(&connection).await?;
    let mut live = Live {
        connection: connection.clone(),
        tx,
        state: SyncHoldState::default(),
        notes: links.notes,
        notifications: links.notifications,
        launch: links.launch,
        posted: Posted::default(),
    };
    owner::follow_owner(
        &connection,
        wire::SYNC,
        &mut live,
        async |live| live.session(&mut commands, &mut answers).await,
        Live::lost,
    )
    .await;
    Ok(())
}

impl Live {
    /// One syncd lifetime: subscribe, join with `Watch`, then follow the signals, the server's
    /// answers and the commands.
    async fn session(
        &mut self,
        commands: &mut UnboundedReceiver<SyncHoldCommand>,
        answers: &mut MessageStream,
    ) -> Ended {
        let Ok(proxy) = SyncProxy::new(&self.connection).await else {
            return Ended::Unreadable;
        };
        // Open before joining, so a hold already on (told before `Watch` returns) is not lost.
        let Ok(mut signals) = proxy.receive_needs_confirmation().await else {
            return Ended::Unreadable;
        };
        if let Err(error) = proxy.watch().await {
            eprintln!("sill: sync hold: joining syncd: {error}");
            return Ended::Unreadable;
        }
        loop {
            tokio::select! {
                Some(signal) = next(&mut signals) => {
                    if let Ok(args) = signal.args() {
                        self.heard(args.dataset(), args.held()).await;
                    }
                }
                Some(Ok(message)) = next(answers) => {
                    if let Some(said) = actions::said(&self.connection, &message).await {
                        self.heard_server(said);
                    }
                }
                Ok(()) = self.notes.changed() => self.learn(),
                command = commands.recv() => match command {
                    Some(SyncHoldCommand::Rewatch) => {
                        if let Err(error) = proxy.watch().await {
                            eprintln!("sill: sync hold: joining syncd: {error}");
                        }
                    }
                    None => return Ended::Commands,
                },
            }
        }
    }

    /// syncd said something about `dataset`: a hold, once the account's name is read, or its end.
    async fn heard(&mut self, dataset: &str, details: &porter_dbus::Details) {
        match wire::heard(dataset, details) {
            Some(Heard::Ended(dataset)) => self.apply(SyncHoldEvent::Ended(dataset)),
            Some(held) => {
                let Heard::Held { account, .. } = &held else {
                    return;
                };
                let name = accounts_wire::label_of(&self.connection, account).await;
                if let Some(hold) = held.hold(name) {
                    self.apply(SyncHoldEvent::Held(hold));
                }
            }
            None => {}
        }
    }

    /// Keep the id of each notice the notes now hold for a dataset that is held.
    fn learn(&mut self) {
        let datasets: Vec<Dataset> = self
            .state
            .holds
            .iter()
            .map(|hold| hold.dataset.clone())
            .collect();
        self.posted.learn(&self.notes.borrow(), &datasets);
    }

    /// The server's word: a press of a notice sill posted opens Settings at the account; a close
    /// ends the notice's claim.
    fn heard_server(&mut self, said: actions::Said) {
        // The notes may already have let go of a notice just pressed.
        self.learn();
        match said {
            actions::Said::Pressed { id, key } => {
                let Some(dataset) = self.posted.answers(id, &key) else {
                    return;
                };
                if let Some(hold) = self.state.holds.iter().find(|h| h.dataset == dataset) {
                    let _ = self
                        .launch
                        .send(LaunchCommand::OpenSettings(SettingsOpen::at(
                            hold.account.clone(),
                        )));
                }
            }
            actions::Said::Closed(id) => self.posted.closed(id),
        }
    }

    /// syncd is gone: what it held is unknown until it is back and joined, so the notices are
    /// taken back and told again from its answer.
    fn lost(&mut self) {
        let datasets: Vec<Dataset> = self
            .state
            .holds
            .iter()
            .map(|hold| hold.dataset.clone())
            .collect();
        for dataset in datasets {
            self.apply(SyncHoldEvent::Ended(dataset));
        }
    }

    /// Step the state by `event`, publish it, and carry out what the step asks.
    fn apply(&mut self, event: SyncHoldEvent) {
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
    }

    fn carry_out(&mut self, effect: SyncHoldEffect) {
        let command = match effect {
            SyncHoldEffect::Tell(hold) => {
                self.posted.forget(&hold.dataset);
                let replaces = shown_for(&self.notes.borrow(), &hold.dataset);
                Some(NotificationsCommand::Post(Box::new(notice::posting(
                    &hold, replaces,
                ))))
            }
            SyncHoldEffect::Withdraw(dataset) => {
                self.posted.forget(&dataset);
                shown_for(&self.notes.borrow(), &dataset).map(NotificationsCommand::Dismiss)
            }
        };
        if let Some(command) = command {
            // A stopped notifications service has nobody to tell.
            let _ = self.notifications.send(command);
        }
    }
}
