//! The pins task: load once, apply every command, save atomically, and pick up an external
//! edit through [`crate::debounced`]. Pins of an app that was renamed are published under its
//! current id; the file follows with the next save.

use std::path::{Path, PathBuf};

use sill_model::apps::AppIndex;
use sill_model::pins::{DockPins, PinEvent, step};
use sill_model::seed::{Store, first_dock};
use tokio::sync::{mpsc, watch};

use super::file::{FILE_NAME, load, save, store};
use crate::debounced::{self, Debounced};
use crate::machine::Published;

/// Run until `commands` closes (the `Control` was dropped). While the file has never been
/// written, wait for the installed apps and start the dock from them (once).
pub(crate) async fn run(
    dir: PathBuf,
    tx: watch::Sender<DockPins>,
    mut commands: mpsc::UnboundedReceiver<PinEvent>,
    mut apps: watch::Receiver<AppIndex>,
) {
    let tx = Published::new(tx, step);
    follow_renames(&tx, &apps.borrow_and_update());
    let mut fresh = (store(&dir) == Store::Missing).then(|| apps.clone());
    let mut touched = match Debounced::file(&dir, FILE_NAME) {
        Ok(touched) => Some(touched),
        Err(error) => {
            eprintln!("sill: watching {}: {error}", dir.display());
            None
        }
    };
    loop {
        let installed = async {
            match &mut fresh {
                Some(apps) => apps
                    .wait_for(|index| !index.apps.is_empty())
                    .await
                    .ok()
                    .map(|index| AppIndex::clone(&index)),
                None => std::future::pending().await,
            }
        };
        tokio::select! {
            event = commands.recv() => {
                let Some(event) = event else { return };
                apply_and_save(&dir, &tx, event);
            }
            settled = debounced::settled(&mut touched) => {
                if settled.is_none() {
                    touched = None;
                    continue;
                }
                reread(&dir, &tx, &apps.borrow());
            }
            changed = apps.changed() => {
                if changed.is_err() {
                    return;
                }
                follow_renames(&tx, &apps.borrow_and_update());
            }
            Some(installed) = installed => {
                fresh = None;
                seed(&dir, &tx, &installed);
            }
        }
    }
}

/// Start the dock from `installed` unless the file was written in the meantime (a pin made
/// before the apps were known).
fn seed(dir: &Path, tx: &Published<DockPins, PinEvent>, installed: &AppIndex) {
    if let Some(pins) = first_dock(store(dir), installed) {
        apply_and_save(dir, tx, PinEvent::Replaced(pins));
    }
}

/// Saved before it is published: whoever sees the new pins finds them on disk too.
fn apply_and_save(dir: &Path, tx: &Published<DockPins, PinEvent>, event: PinEvent) {
    let next = tx.next(event);
    if let Err(error) = save(dir, &next) {
        eprintln!("sill: saving {}: {error}", dir.display());
    }
    tx.set(next);
}

/// An external edit settled: re-read and publish only if it actually changed the value (our
/// own writes trigger the same watch; re-reading them back is a no-op here).
fn reread(dir: &Path, tx: &Published<DockPins, PinEvent>, installed: &AppIndex) {
    tx.set(load(dir).renamed(installed));
}

/// The published pins with the ids of renamed apps made current. The file keeps the old ids
/// until the next save, which writes what is published.
fn follow_renames(tx: &Published<DockPins, PinEvent>, installed: &AppIndex) {
    tx.set(tx.borrow().clone().renamed(installed));
}
