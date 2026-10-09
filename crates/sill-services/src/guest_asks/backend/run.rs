//! The guest-asks loop: inferd's `GuestAsks` and `GuestsChanged` signals, the notifications
//! server's answers and the commands, stepping [`GuestAsksState`], posting its notices and
//! giving inferd the person's answers.

use porter_dbus::{COMPUTER_ERROR_PREFIX, InferenceProxy};
use sill_model::guest_asks::{
    Ask, Computer, GuestAsksCommand, GuestAsksEffect, GuestAsksEvent, GuestAsksState, Posted,
    Verdict, shown_for, step,
};
use sill_model::notifications::{NotesState, NotificationsCommand};
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::sync::watch;
use zbus::{Connection, MessageStream};

use super::links::GuestAsksLinks;
use super::notice;
use super::wire;
use crate::accounts::actions;
use crate::control::Control;
use crate::env::Bus;
use crate::error::ServiceError;
use crate::guest_asks::GuestAsksService;
use crate::owner::{self, Ended};
use crate::service::ServiceIo;
use crate::stream::next;

/// The error inferd answers when someone else (Settings) already answered the question.
const NOT_ASKING: &str = "NotAsking";

/// Everything the loop owns.
struct Live {
    connection: Connection,
    tx: watch::Sender<GuestAsksState>,
    state: GuestAsksState,
    notes: watch::Receiver<NotesState>,
    notifications: Control<NotificationsCommand>,
    /// The ids of the notices sill posted, the only ones whose buttons answer.
    posted: Posted,
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
    let connection = bus.connected().await?;
    let mut answers = actions::stream(&connection).await?;
    let mut live = Live {
        connection: connection.clone(),
        tx,
        state: GuestAsksState::default(),
        notes: links.notes,
        notifications: links.notifications,
        posted: Posted::default(),
    };
    loop {
        owner::appeared(&connection, wire::INFERD).await;
        tokio::select! {
            met = live.session(&mut commands, &mut answers) => match met {
                Ended::Commands => return Ok(()),
                Ended::Unreadable => owner::changed(&connection, wire::INFERD).await,
            },
            () = owner::changed(&connection, wire::INFERD) => {}
        }
        live.lost();
    }
}

impl Live {
    /// One inferd lifetime: subscribe, read what is waiting, then follow the signals, the
    /// server's answers and the commands.
    async fn session(
        &mut self,
        commands: &mut UnboundedReceiver<GuestAsksCommand>,
        answers: &mut MessageStream,
    ) -> Ended {
        let Ok(proxy) = InferenceProxy::new(&self.connection).await else {
            return Ended::Unreadable;
        };
        // Open before reading, so a question asked in between is not lost.
        let Ok(mut asks) = proxy.receive_guest_asks().await else {
            return Ended::Unreadable;
        };
        let Ok(mut changes) = proxy.receive_guests_changed().await else {
            return Ended::Unreadable;
        };
        if !self.reread(&proxy).await {
            return Ended::Unreadable;
        }
        loop {
            tokio::select! {
                Some(signal) = next(&mut asks) => {
                    let ask = signal
                        .args()
                        .ok()
                        .and_then(|args| wire::asked(args.node(), args.details()));
                    if let Some(ask) = ask {
                        self.apply(GuestAsksEvent::Asked(ask), &proxy).await;
                    }
                }
                Some(_) = next(&mut changes) => {
                    self.reread(&proxy).await;
                }
                Some(Ok(message)) = next(answers) => {
                    if let Some(said) = actions::said(&self.connection, &message).await {
                        self.heard_server(said, &proxy).await;
                    }
                }
                Ok(()) = self.notes.changed() => self.learn(),
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
    async fn reread(&mut self, proxy: &InferenceProxy<'_>) -> bool {
        match proxy.guests().await {
            Ok(rows) => {
                self.apply(GuestAsksEvent::Waiting(wire::waiting(&rows)), proxy)
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
        self.posted.learn(&self.notes.borrow(), &computers);
    }

    /// The server's word: a press of a notice sill posted answers the question; a close ends the
    /// notice's claim.
    async fn heard_server(&mut self, said: actions::Said, proxy: &InferenceProxy<'_>) {
        // The notes may already have let go of a notice just pressed.
        self.learn();
        match said {
            actions::Said::Pressed { id, key } => {
                if let Some((node, verdict)) = self.posted.answers(id, &key) {
                    self.apply(GuestAsksEvent::Chosen(node, verdict), proxy)
                        .await;
                }
            }
            actions::Said::Closed(id) => self.posted.closed(id),
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
    async fn apply(&mut self, event: GuestAsksEvent, proxy: &InferenceProxy<'_>) {
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
        self.posted.forget(&ask.node);
        let replaces = shown_for(&self.notes.borrow(), &ask.node);
        // A stopped notifications service has nobody to tell.
        let _ = self
            .notifications
            .send(NotificationsCommand::Post(Box::new(notice::posting(
                ask, replaces,
            ))));
    }

    fn withdraw(&mut self, node: &Computer) {
        self.posted.forget(node);
        if let Some(id) = shown_for(&self.notes.borrow(), node) {
            let _ = self.notifications.send(NotificationsCommand::Dismiss(id));
        }
    }
}

/// Give inferd the person's answer. Someone else having answered first is not worth a word.
async fn answer(proxy: &InferenceProxy<'_>, node: &Computer, verdict: Verdict) {
    let Err(error) = proxy.answer_guest(node.as_str(), wire::word(verdict)).await else {
        return;
    };
    let already = matches!(
        &error,
        zbus::Error::MethodError(name, _, _)
            if name.as_str() == format!("{COMPUTER_ERROR_PREFIX}{NOT_ASKING}")
    );
    if !already {
        eprintln!("sill: guest asks: answering {}: {error}", node.as_str());
    }
}
