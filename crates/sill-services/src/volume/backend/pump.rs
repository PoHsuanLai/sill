//! Carries the service's commands to the thread that owns the connection to the sound server, and
//! rings the thread awake for each: the thread sleeps in libpulse's poll, which watches the
//! descriptor of a [`Wake`], so an idle desktop costs it no wakeups at all.

use std::sync::Arc;

use sill_model::volume::{Volume, VolumeCommand};
use tokio::sync::{mpsc, watch};

use super::live;
use super::wake::Wake;

/// Runs until `commands` closes (every `Control` was dropped): the blocking thread on the
/// blocking pool, and the commands from the async side to it.
pub(crate) async fn run(
    tx: watch::Sender<Volume>,
    mut commands: mpsc::UnboundedReceiver<VolumeCommand>,
) {
    let wake = match Wake::new() {
        Ok(wake) => Arc::new(wake),
        Err(error) => {
            eprintln!("sill: volume: no way to wake the sound thread: {error}");
            return;
        }
    };
    let (queue, queued) = std::sync::mpsc::channel();
    let thread = tokio::task::spawn_blocking({
        let wake = Arc::clone(&wake);
        move || live::run(&tx, &queued, &wake)
    });
    while let Some(command) = commands.recv().await {
        if queue.send(command).is_err() {
            break;
        }
        wake.ring();
    }
    // Closing the queue is how the thread learns it is done; the ring makes it look.
    drop(queue);
    wake.ring();
    let _ = thread.await;
}
