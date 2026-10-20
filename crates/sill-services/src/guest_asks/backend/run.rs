//! The guest-asks loop: inferd's `GuestAsks` and `GuestsChanged` signals, the notifications
//! server's answers and the commands, stepping [`GuestAsksState`], posting its notices and
//! giving inferd the person's answers.

use porter_client::{
    Accounts, ClientError, ComputerReason, DbusTransport, GuestChange, NodeId, TransportError,
};
use sill_model::guest_asks::{
    Ask, Computer, GuestAsksCommand, GuestAsksEffect, GuestAsksEvent, GuestAsksState, Verdict,
    choice_of, shown_for, step,
};
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::sync::watch;
use zbus::{Connection, MessageStream};

use super::links::GuestAsksLinks;
use super::notice;
use super::wire;
use crate::env::Bus;
use crate::error::ServiceError;
use crate::guest_asks::GuestAsksService;
use crate::notices::{self, Board, Said};
use crate::owner::{self, Ended};
use crate::service::ServiceIo;
use crate::stream::next;

/// The shell's seat at inferd's guests.
type Guests = Accounts<DbusTransport>;

/// Everything the loop owns.
struct Live {
    connection: Connection,
    tx: watch::Sender<GuestAsksState>,
    state: GuestAsksState,
    /// The questions, one notice per asking computer.
    board: Board<Computer>,
}

/// Runs until the commands close. Without inferd the state stays empty; inferd starting late,
/// leaving or restarting is followed through `NameOwnerChanged`, and each new owner's waiting
/// questions are read again.
pub(crate) async fn run(
    bus: Bus,
    links: GuestAsksLinks,
    io: ServiceIo<GuestAsksService>,
) -> Result<(), ServiceError> {
    let ServiceIo {
        state: tx,
        mut commands,
    } = io;
    let connection = bus.connect_waiting().await?;
    let mut answers = notices::stream(&connection).await?;
    let mut live = Live {
        connection: connection.clone(),
        tx,
        state: GuestAsksState::default(),
        board: Board::new(links.notifications, links.notes, shown_for),
    };
    owner::follow_owner(
        &connection,
        wire::INFERD,
        &mut live,
        async |live| live.session(&mut commands, &mut answers).await,
        Live::lost,
    )
    .await;
    Ok(())
}

impl Live {
    /// One inferd lifetime: subscribe, read what is waiting, then follow the signals, the
    /// server's answers and the commands.
    async fn session(
        &mut self,
        commands: &mut UnboundedReceiver<GuestAsksCommand>,
        answers: &mut MessageStream,
    ) -> Ended {
        let proxy = Accounts::over(DbusTransport::over(self.connection.clone()));
        // Open before reading, so a question asked in between is not lost.
        let Ok(mut changes) = proxy.watch_guests().await else {
            return Ended::Unreadable;
        };
        if !self.reread(&proxy).await {
            return Ended::Unreadable;
        }
        loop {
            tokio::select! {
                Some(change) = changes.next() => match change {
                    GuestChange::Asks { node, ask } => {
                        if let Some(ask) = wire::asked(&node, ask) {
                            self.apply(GuestAsksEvent::Asked(ask), &proxy).await;
                        }
                    }
                    _ => {
                        self.reread(&proxy).await;
                    }
                },
                Some(Ok(message)) = next(answers) => {
                    if let Some(said) = notices::said(&self.connection, &message).await {
                        self.heard_server(said, &proxy).await;
                    }
                }
                Ok(()) = self.board.changed() => self.learn(),
                command = commands.recv() => match command {
                    Some(GuestAsksCommand::Reread) => {
                        self.reread(&proxy).await;
                    }
                    None => return Ended::Commands,
                },
            }
        }
    }

    /// Read `Guests` and take the questions waiting from it; whether inferd answered.
    async fn reread(&mut self, proxy: &Guests) -> bool {
        match proxy.guests().await {
            Ok(rows) => {
                self.apply(GuestAsksEvent::Waiting(wire::waiting(rows)), proxy)
                    .await;
                true
            }
            Err(error) => {
                eprintln!("sill: guest asks: reading the questions: {error}");
                false
            }
        }
    }

    /// Keep the id of each notice the notes now hold for a computer that is asking.
    fn learn(&mut self) {
        let computers: Vec<Computer> = self.state.asks.iter().map(|a| a.node.clone()).collect();
        self.board.learn(&computers);
    }

    /// The server's word: a press of a notice sill posted answers the question; a close ends the
    /// notice's claim.
    async fn heard_server(&mut self, said: Said, proxy: &Guests) {
        // The notes may already have let go of a notice just pressed.
        self.learn();
        match said {
            Said::Pressed { id, key } => {
                if let Some((node, verdict)) =
                    self.board.answers(id, choice_of(&key), |(node, _)| node)
                {
                    self.apply(GuestAsksEvent::Chosen(node, verdict), proxy)
                        .await;
                }
            }
            Said::Closed(id) => self.board.closed(id),
        }
    }

    /// inferd is gone: what it was asking is unknown until it is back, so the notices are taken
    /// back and told again from its answer.
    fn lost(&mut self) {
        let (next, effects) = step(
            std::mem::take(&mut self.state),
            GuestAsksEvent::Waiting(Vec::new()),
        );
        self.publish(next);
        for effect in effects {
            if let GuestAsksEffect::Withdraw(node) = effect {
                self.withdraw(&node);
            }
        }
    }

    /// Step the state by `event`, publish it, and carry out what the step asks.
    async fn apply(&mut self, event: GuestAsksEvent, proxy: &Guests) {
        let (next, effects) = step(std::mem::take(&mut self.state), event);
        self.publish(next);
        for effect in effects {
            match effect {
                GuestAsksEffect::Tell(ask) => self.tell(&ask),
                GuestAsksEffect::Withdraw(node) => self.withdraw(&node),
                GuestAsksEffect::Answer(node, verdict) => answer(proxy, &node, verdict).await,
            }
        }
    }

    fn publish(&mut self, next: GuestAsksState) {
        self.tx.send_if_modified(|published| {
            let changed = *published != next;
            if changed {
                published.clone_from(&next);
            }
            changed
        });
        self.state = next;
    }

    fn tell(&mut self, ask: &Ask) {
        self.board
            .post(&ask.node, |replaces| notice::posting(ask, replaces));
    }

    fn withdraw(&mut self, node: &Computer) {
        self.board.withdraw(node);
    }
}

/// Give inferd the person's answer. Someone else having answered first is not worth a word; any
/// other refusal is printed in inferd's own words.
async fn answer(proxy: &Guests, node: &Computer, verdict: Verdict) {
    let Ok(id) = NodeId::parse(node.as_str()) else {
        eprintln!("sill: guest asks: {} is not a computer id", node.as_str());
        return;
    };
    let Err(error) = proxy.answer_guest(&id, wire::answer(verdict)).await else {
        return;
    };
    let already = matches!(
        &error,
        ClientError::Transport(TransportError::Computer {
            reason: ComputerReason::NotAsking,
            ..
        })
    );
    if !already {
        eprintln!("sill: guest asks: answering {}: {error}", node.as_str());
    }
}
