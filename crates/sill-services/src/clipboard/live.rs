//! The service's task: it owns the history, reduces every command and every event of its
//! Wayland thread ([`super::wire`]) through [`step`], publishes the result, and
//! carries out the effects: `Connect` starts a thread (its own connection, `ext` else `wlr` on
//! the first seat), `Disconnect` drops it, `OfferSelection` hands the entry to it. Nothing
//! connects until a policy turns history on, so `Services::spawn` in a test touches no
//! compositor. Only counts and fault kinds are ever printed.

use tokio::sync::{mpsc, watch};

use super::model::{ClipEffect, ClipEvent, ClipState, Clipboard, ClipboardCommand};
use super::reduce::step;
use super::wire::{Epoch, Heard, Order, Tell, Wire, start};
use crate::env::WaylandRoute;

/// Run until every control is dropped.
pub(super) async fn run(
    display: WaylandRoute,
    mut commands: mpsc::UnboundedReceiver<ClipboardCommand>,
    tx: watch::Sender<Clipboard>,
) {
    let (heard, mut hearing) = mpsc::unbounded_channel();
    let mut live = Live {
        display,
        state: ClipState::default(),
        wire: None,
        epoch: Epoch(0),
        heard,
    };
    loop {
        let event = tokio::select! {
            command = commands.recv() => match command {
                Some(command) => ClipEvent::Command(command),
                None => break,
            },
            Some(Heard { epoch, event }) = hearing.recv() => {
                if epoch != live.epoch {
                    continue;
                }
                event
            }
        };
        live.step(event, &tx);
    }
}

/// The task's state: the reducer's, and the thread of the current connection.
struct Live {
    /// The compositor (a live test's nested one by socket).
    display: WaylandRoute,
    state: ClipState,
    wire: Option<Wire>,
    /// The current connection's; events of an older one are dropped.
    epoch: Epoch,
    heard: mpsc::UnboundedSender<Heard>,
}

impl Live {
    fn step(&mut self, event: ClipEvent, tx: &watch::Sender<Clipboard>) {
        let images = self.state.policy.images;
        let (state, effects) = step(std::mem::take(&mut self.state), event);
        self.state = state;
        if self.state.policy.images != images
            && let Some(wire) = &self.wire
        {
            wire.order(Order::Images(self.state.policy.images));
        }
        for effect in effects {
            self.carry_out(effect, tx);
        }
        publish(tx, &self.state.shown);
    }

    fn carry_out(&mut self, effect: ClipEffect, tx: &watch::Sender<Clipboard>) {
        match effect {
            ClipEffect::Connect => {
                self.epoch = Epoch(self.epoch.0 + 1);
                let tell = Tell::new(self.epoch, self.heard.clone());
                match start(self.display.clone(), self.state.policy.images, tell) {
                    Ok(wire) => self.wire = Some(wire),
                    Err(error) => eprintln!("sill: clipboard: no thread: {}", error.kind()),
                }
            }
            ClipEffect::Disconnect => {
                self.wire = None;
                self.epoch = Epoch(self.epoch.0 + 1);
                self.step(ClipEvent::Disconnected, tx);
            }
            ClipEffect::OfferSelection(entry) => {
                if let Some(wire) = &self.wire {
                    wire.order(Order::Offer(entry.content));
                }
            }
        }
    }
}

/// `shown` to the launcher and `sill clipboard status`, when it changed.
fn publish(tx: &watch::Sender<Clipboard>, shown: &Clipboard) {
    tx.send_if_modified(|published| {
        let changed = published != shown;
        if changed {
            published.clone_from(shown);
        }
        changed
    });
}
