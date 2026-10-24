//! The frecency history every part of the search shares: the runner records choices into it
//! and reads it for Recent, the apps provider ranks with it (`AppsProvider::frecency`), and a
//! saver task writes each new version to `frecency.json`, one at a time, newest wins.

use std::path::PathBuf;
use std::sync::{Arc, Mutex, PoisonError};

use sill_launcher::{AppId, Frecency, ItemId, ItemKey, ProviderKind, UnixTime};
use sill_model::apps::AppIndex;
use tokio::sync::watch;

use super::frecency_file::save_frecency;

/// One shared, in-memory history.
#[derive(Debug, Clone, Default)]
pub(crate) struct History(Arc<Mutex<Frecency>>);

impl History {
    /// A history starting from `frecency` (the file's contents).
    pub(crate) fn new(frecency: Frecency) -> History {
        History(Arc::new(Mutex::new(frecency)))
    }

    /// The handle a provider ranks with; it sees every later [`History::record`].
    pub(crate) fn shared(&self) -> Arc<Mutex<Frecency>> {
        Arc::clone(&self.0)
    }

    /// `read` applied to the history as it is now.
    pub(crate) fn read<T>(&self, read: impl FnOnce(&Frecency) -> T) -> T {
        // A provider that panicked mid-read cannot have left a half-written history: only
        // `record` writes, and it replaces the value whole.
        read(&self.0.lock().unwrap_or_else(PoisonError::into_inner))
    }

    /// Whether nothing has been counted.
    pub(crate) fn untouched(&self) -> bool {
        self.read(|frecency| frecency.entries.is_empty())
    }

    /// Start the history from `seed`, which replaces what is there; the history as it now stands.
    pub(crate) fn start_from(&self, seed: Frecency) -> Frecency {
        let mut guard = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        *guard = seed;
        guard.clone()
    }

    /// Move the uses of an app that was renamed (`X-Flatpak-RenamedFrom`) to its current id. The
    /// file follows with the next save, which writes what the history holds then.
    pub(crate) fn rename_apps(&self, installed: &AppIndex) {
        let renames = installed.renames();
        if renames.is_empty() {
            return;
        }
        let mut guard = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        *guard = std::mem::take(&mut *guard).rekeyed(ProviderKind::Apps, |item| {
            let now = renames.of(&AppId(item.0.clone()));
            (now.0 != item.0).then_some(ItemId(now.0))
        });
    }

    /// Count one more use of `key` at `now`; the history as it now stands.
    pub(crate) fn record(&self, key: ItemKey, now: UnixTime) -> Frecency {
        let mut guard = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        *guard = std::mem::take(&mut *guard).record(key, now);
        guard.clone()
    }
}

/// Where new versions of the history go to be written.
#[derive(Debug)]
pub(crate) enum Persist {
    /// To the saver task writing `dir/frecency.json`.
    To(watch::Sender<Frecency>),
    /// Nowhere: no state directory, so choices count for this session only.
    Nowhere,
}

impl Persist {
    /// Start a saver for `dir` (on the current runtime), or none.
    pub(crate) fn start(dir: Option<PathBuf>, initial: Frecency) -> Persist {
        let Some(dir) = dir else {
            eprintln!("sill: no state directory; launcher frecency is not persisted");
            return Persist::Nowhere;
        };
        let (tx, rx) = watch::channel(initial);
        tokio::spawn(save_latest(dir, rx));
        Persist::To(tx)
    }

    /// Queue `frecency` to be written; a version not yet written is superseded.
    pub(crate) fn save(&self, frecency: Frecency) {
        if let Persist::To(tx) = self {
            tx.send_replace(frecency);
        }
    }
}

/// Write each new version as it arrives, one write at a time, until the runner is gone.
async fn save_latest(dir: PathBuf, mut versions: watch::Receiver<Frecency>) {
    while versions.changed().await.is_ok() {
        let frecency = versions.borrow_and_update().clone();
        let at = dir.clone();
        let written = tokio::task::spawn_blocking(move || save_frecency(&at, &frecency)).await;
        match written {
            Ok(Ok(())) => {}
            Ok(Err(error)) => eprintln!("sill: saving frecency: {error}"),
            Err(error) => eprintln!("sill: saving frecency: {error}"),
        }
    }
}
