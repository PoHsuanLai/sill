//! The events task's `notify` watch over a provider's `watched()` places, debounced: a file is
//! watched through its directory (an editor's or vdirsyncer's atomic save replaces the file),
//! a directory recursively (a vdir's collections).

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::time::Duration;

use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use tokio::sync::mpsc;

/// How long the places must be quiet before a change is reported settled.
pub(super) const DEBOUNCE: Duration = Duration::from_millis(150);

/// A running watch; dropping it stops watching.
pub(super) struct Touches {
    signals: mpsc::UnboundedReceiver<()>,
    _watcher: Option<RecommendedWatcher>,
}

impl Touches {
    /// Watch `places` (those that do not exist, or cannot be watched, are left out).
    pub fn watch(places: &[PathBuf]) -> Touches {
        let (signal, signals) = mpsc::unbounded_channel::<()>();
        let targets: Vec<Target> = places.iter().map(|path| Target::of(path)).collect();
        let wanted = targets.clone();
        let watcher = notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
            if let Ok(event) = res
                && !event.kind.is_access()
                && event
                    .paths
                    .iter()
                    .any(|path| wanted.iter().any(|t| t.covers(path)))
            {
                // A dropped signal means the task ended: nobody is left to re-read for.
                let _ = signal.send(());
            }
        });
        let watcher = watcher.ok().map(|mut watcher| {
            for target in &targets {
                if let Err(error) = watcher.watch(&target.dir, target.mode) {
                    eprintln!("sill: events: watching {}: {error}", target.dir.display());
                }
            }
            watcher
        });
        Touches {
            signals,
            _watcher: watcher,
        }
    }

    /// The next settled change: the first signal, then quiet for [`DEBOUNCE`]. Pending forever
    /// once the watcher is gone.
    pub async fn settled(&mut self) {
        if self.signals.recv().await.is_none() {
            std::future::pending::<()>().await;
        }
        while let Ok(Some(())) = tokio::time::timeout(DEBOUNCE, self.signals.recv()).await {}
    }
}

/// One place's watch: the directory watched and, for a file, the name that matters in it.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Target {
    dir: PathBuf,
    mode: RecursiveMode,
    file: Option<OsString>,
}

impl Target {
    fn of(path: &Path) -> Target {
        match (path.is_dir(), path.parent(), path.file_name()) {
            (false, Some(parent), Some(name)) => Target {
                dir: parent.to_path_buf(),
                mode: RecursiveMode::NonRecursive,
                file: Some(name.to_owned()),
            },
            _ => Target {
                dir: path.to_path_buf(),
                mode: RecursiveMode::Recursive,
                file: None,
            },
        }
    }

    /// Whether a change at `path` is one this place cares about.
    fn covers(&self, path: &Path) -> bool {
        match &self.file {
            Some(name) => {
                path.parent() == Some(self.dir.as_path()) && path.file_name() == Some(name)
            }
            None => path.starts_with(&self.dir),
        }
    }
}
