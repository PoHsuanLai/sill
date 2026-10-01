//! A directory watch on one file in it: 30 ms debounce, so an external edit (or our own atomic
//! write) shows up as one settled touch rather than a burst of rename events.

use std::ffi::{OsStr, OsString};
use std::path::Path;
use std::time::Duration;

use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use tokio::sync::mpsc;

use crate::error::ServiceError;

/// How long the directory must be quiet before a touch is reported settled.
pub const DEBOUNCE: Duration = Duration::from_millis(30);

/// A running watch on one file of a directory.
pub struct FileTouch {
    signals: mpsc::UnboundedReceiver<()>,
    _watcher: RecommendedWatcher,
}

impl FileTouch {
    /// Wait for the next settled touch: the first signal, then every further signal within
    /// [`DEBOUNCE`] restarts the wait. `None` once the watcher itself is gone.
    pub async fn settled(&mut self) -> Option<()> {
        self.signals.recv().await?;
        loop {
            match tokio::time::timeout(DEBOUNCE, self.signals.recv()).await {
                Ok(Some(())) => continue,
                Ok(None) => return None,
                Err(_elapsed) => return Some(()),
            }
        }
    }
}

/// Whether `event` touches the file called `name` itself: not its temp file, not a sibling, and
/// not a mere `Access` (counting a re-read as a change would re-read forever).
fn touches_the_file(event: &notify::Event, name: &OsStr) -> bool {
    !event.kind.is_access()
        && event
            .paths
            .iter()
            .any(|path| path.file_name() == Some(name))
}

/// Start watching `dir` (created if missing) for changes to its file `name`. Needs a Tokio
/// runtime (the watcher's callback sends into an unbounded channel; nothing here blocks it).
pub fn watch(dir: &Path, name: &str) -> Result<FileTouch, ServiceError> {
    let name = OsString::from(name);
    std::fs::create_dir_all(dir).map_err(|source| ServiceError::Io {
        path: dir.to_path_buf(),
        source,
    })?;
    let (signal, signals) = mpsc::unbounded_channel::<()>();
    let mut watcher = notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
        if let Ok(event) = res
            && touches_the_file(&event, &name)
        {
            // A dropped signal means the watch task ended: nobody is left to re-read for.
            let _ = signal.send(());
        }
    })?;
    watcher.watch(dir, RecursiveMode::NonRecursive)?;
    Ok(FileTouch {
        signals,
        _watcher: watcher,
    })
}

#[cfg(test)]
mod tests {
    use super::watch;
    use sill_testkit::TempDir;

    const FILE_NAME: &str = "dock.json";

    /// A rename onto the file (the atomic writer's own move) settles as exactly one touch,
    /// and a change to an unrelated sibling file is not reported at all.
    #[tokio::test]
    async fn watch_fires_once_on_rename_and_ignores_siblings() {
        let dir = TempDir::new("fs-watch");
        let mut touched = watch(dir.path(), FILE_NAME).unwrap_or_else(|e| panic!("{e}"));

        let tmp = dir.path().join("dock.json.part");
        std::fs::write(&tmp, b"{}").unwrap_or_else(|e| panic!("{e}"));
        std::fs::rename(&tmp, dir.path().join(FILE_NAME)).unwrap_or_else(|e| panic!("{e}"));

        let settled = tokio::time::timeout(std::time::Duration::from_secs(2), touched.settled())
            .await
            .unwrap_or_else(|_| panic!("no touch reported within 2s"));
        assert_eq!(settled, Some(()));

        std::fs::write(dir.path().join("unrelated"), b"x").unwrap_or_else(|e| panic!("{e}"));
        let extra = tokio::time::timeout(super::DEBOUNCE * 4, touched.settled()).await;
        assert!(extra.is_err(), "a sibling file must not be reported");
    }
}
