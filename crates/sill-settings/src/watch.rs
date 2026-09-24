//! Live reload (design/22 §2): a `notify` watch on the config directory (editors and the atomic
//! writer replace the inode by rename), 30 ms debounce, the whole file re-read on each settled
//! change. The same mechanism as `ds_settings::watch`, which is fixed to `appearance.toml`.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use ds_settings::{DEBOUNCE, SettingsError};
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use tokio::sync::watch as channel;

use crate::file::{FILE_NAME, ShellFile, load};

/// A running watch on one directory's `settings.toml`. Needs a Tokio runtime.
#[derive(Debug)]
pub struct ShellWatch {
    changes: channel::Receiver<ShellFile>,
    _watcher: RecommendedWatcher,
}

impl ShellWatch {
    /// Wait for the next settled change; the whole re-read file. `None` once stopped.
    pub async fn changed(&mut self) -> Option<ShellFile> {
        self.changes.changed().await.ok()?;
        Some(self.changes.borrow_and_update().clone())
    }

    /// The file as last read.
    pub fn current(&self) -> ShellFile {
        self.changes.borrow().clone()
    }

    /// A receiver of every settled file, for [`crate::SettingsFeed`].
    pub fn receiver(&self) -> channel::Receiver<ShellFile> {
        self.changes.clone()
    }
}

/// Whether `event` changed `settings.toml` itself: not its temp file, not a sibling, and not an
/// `Access` (a re-read is not a change; counting it would re-read forever).
fn touches_the_file(event: &notify::Event) -> bool {
    !event.kind.is_access()
        && event
            .paths
            .iter()
            .any(|path| path.file_name() == Some(OsStr::new(FILE_NAME)))
}

/// Start watching `dir` (created if missing) on the current Tokio runtime.
pub fn watch(dir: &Path) -> Result<ShellWatch, SettingsError> {
    std::fs::create_dir_all(dir).map_err(|source| SettingsError::Io {
        path: dir.to_path_buf(),
        source,
    })?;
    let (signal, mut signals) = tokio::sync::mpsc::unbounded_channel::<()>();
    let mut watcher = notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
        if let Ok(event) = res
            && touches_the_file(&event)
        {
            // A dropped signal means the watch task ended: nobody is left to re-read for.
            let _ = signal.send(());
        }
    })?;
    watcher.watch(dir, RecursiveMode::NonRecursive)?;

    let (changes_tx, changes) = channel::channel(load(dir));
    let dir: PathBuf = dir.to_path_buf();
    tokio::spawn(async move {
        while signals.recv().await.is_some() {
            // Coalesce the rest of the burst; every further signal restarts the window.
            loop {
                match tokio::time::timeout(DEBOUNCE, signals.recv()).await {
                    Ok(Some(())) => continue,
                    Ok(None) => return,
                    Err(_settled) => break,
                }
            }
            if changes_tx.send(load(&dir)).is_err() {
                return;
            }
        }
    });
    Ok(ShellWatch {
        changes,
        _watcher: watcher,
    })
}
