//! The pins task: load once, apply every command, save atomically, and pick up an external
//! edit through [`fs_watch`].

use std::path::{Path, PathBuf};

use tokio::sync::{mpsc, watch};

use super::fs_watch;
use super::model::{DockPins, PinEvent, step};
use super::{load, save};

/// Run until `commands` closes (the `Control` was dropped).
pub(super) async fn run(
    dir: PathBuf,
    tx: watch::Sender<DockPins>,
    mut commands: mpsc::UnboundedReceiver<PinEvent>,
) {
    let mut touched = match fs_watch::watch(&dir) {
        Ok(touched) => Some(touched),
        Err(error) => {
            eprintln!("sill: watching {}: {error}", dir.display());
            None
        }
    };
    loop {
        let settled = async {
            match &mut touched {
                Some(touched) => touched.settled().await,
                None => std::future::pending().await,
            }
        };
        tokio::select! {
            event = commands.recv() => {
                let Some(event) = event else { return };
                apply_and_save(&dir, &tx, event);
            }
            settled = settled => {
                if settled.is_none() {
                    touched = None;
                    continue;
                }
                reread(&dir, &tx);
            }
        }
    }
}

fn apply(tx: &watch::Sender<DockPins>, event: PinEvent) {
    tx.send_modify(|pins| *pins = step(std::mem::take(pins), event));
}

fn apply_and_save(dir: &Path, tx: &watch::Sender<DockPins>, event: PinEvent) {
    apply(tx, event);
    if let Err(error) = save(dir, &tx.borrow()) {
        eprintln!("sill: saving {}: {error}", dir.display());
    }
}

/// An external edit settled: re-read and publish only if it actually changed the value (our
/// own writes trigger the same watch; re-reading them back is a no-op here).
fn reread(dir: &Path, tx: &watch::Sender<DockPins>) {
    let found = load(dir);
    tx.send_if_modified(|pins| {
        let changed = *pins != found;
        if changed {
            *pins = found;
        }
        changed
    });
}
