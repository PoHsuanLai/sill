//! The sync-hold loop: syncd's `NeedsConfirmation` signals, the notifications server's answers
//! and the commands, stepping [`SyncHoldState`] and posting its notices.

use porter_dbus::sync::SyncProxy;
use sill_model::launch::{LaunchCommand, SettingsOpen};
use sill_model::notifications::Said;
use sill_model::sync_hold::{
    Dataset, SyncHoldCommand, SyncHoldEffect, SyncHoldEvent, SyncHoldState, dataset_of, shown_for,
    step,
};
use tokio::sync::broadcast;
use tokio::sync::mpsc::UnboundedReceiver;
use zbus::Connection;

use super::links::SyncHoldLinks;
use super::notice;
use super::wire::{self, Heard};
use crate::account_label::label_of;
use crate::control::Control;
use crate::env::Bus;
use crate::error::ServiceError;
use crate::machine::Machine;
use crate::notices::Board;
use crate::owner::{self, Ended};
use crate::service::ServiceIo;
use crate::stream::next;
use crate::sync_hold::SyncHoldService;

/// Everything the loop owns.
struct Live {
    connection: Connection,
    machine: Machine<SyncHoldState, SyncHoldEvent, SyncHoldEffect>,
    launch: Control<LaunchCommand>,
    /// The notices, one per held dataset.
    board: Board<Dataset>,
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
    // Listening before the bus is reached, so no answer falls between.
    let mut answers = links.answers.hear();
    let connection = bus.connect_waiting().await?;
    let mut live = Live {
        connection: connection.clone(),
        machine: Machine::new(tx, step),
        launch: links.launch,
        board: Board::new(links.notifications, links.notes, shown_for),
    };
    let owner = owner::Owner::of(&connection, wire::SYNC);
    while owner
        .session(live.session(&mut commands, &mut answers))
        .await
        == owner::Next::Again
    {
        live.lost();
    }
    Ok(())
}

impl Live {
    /// One syncd lifetime: subscribe, join with `Watch`, then follow the signals, the server's
    /// answers and the commands.
    async fn session(
        &mut self,
        commands: &mut UnboundedReceiver<SyncHoldCommand>,
        answers: &mut broadcast::Receiver<Said>,
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
                Ok(said) = answers.recv() => self.heard_server(said),
                Ok(()) = self.board.changed() => self.learn(),
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
                let name = label_of(&self.connection, account).await;
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
            .machine
            .state()
            .holds
            .iter()
            .map(|hold| hold.dataset.clone())
            .collect();
        self.board.learn(&datasets);
    }

    /// The server's word: a press of a notice sill posted opens Settings at the account; a close
    /// ends the notice's claim.
    fn heard_server(&mut self, said: Said) {
        // The notes may already have let go of a notice just pressed.
        self.learn();
        match said {
            Said::Pressed { id, key } => {
                let Some(dataset) = self.board.answers(id, dataset_of(&key), |dataset| dataset)
                else {
                    return;
                };
                if let Some(hold) = self
                    .machine
                    .state()
                    .holds
                    .iter()
                    .find(|h| h.dataset == dataset)
                {
                    let _ = self
                        .launch
                        .send(LaunchCommand::OpenSettings(SettingsOpen::at(
                            hold.account.clone(),
                        )));
                }
            }
            Said::Closed { id, .. } => self.board.closed(id),
        }
    }

    /// syncd is gone: what it held is unknown until it is back and joined, so the notices are
    /// taken back and told again from its answer.
    fn lost(&mut self) {
        let datasets: Vec<Dataset> = self
            .machine
            .state()
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
        for effect in self.machine.feed(event) {
            self.carry_out(effect);
        }
    }

    fn carry_out(&mut self, effect: SyncHoldEffect) {
        match effect {
            SyncHoldEffect::Tell(hold) => self
                .board
                .post(&hold.dataset, |replaces| notice::posting(&hold, replaces)),
            SyncHoldEffect::Withdraw(dataset) => self.board.withdraw(&dataset),
        }
    }
}
