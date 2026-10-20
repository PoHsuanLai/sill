//! Watching files and folders for changes, and reporting a burst of events as one settled touch.
//! The shell's file watchers (its own config files, the wallpaper's source, the applications
//! folders, the calendars) differ only in what they watch, which events count and how long the
//! quiet must last; the rest is here once, including the rule that cost the apps service a busy
//! loop: a file merely read must never count as a change.

use std::path::{Path, PathBuf};
use std::time::Duration;

use notify::event::{AccessKind, AccessMode};
use notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use tokio::sync::mpsc;

use crate::error::ServiceError;

/// How long a file of the shell's own must be quiet before a touch is reported settled: short,
/// so an external edit (or our own atomic write) shows up as one touch, not a burst of renames.
pub(crate) const OWN_FILE_QUIET: Duration = Duration::from_millis(30);

/// Which events count as a change.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EventFilter {
    /// Anything but an access: reading a file, opening it or closing it is never a change.
    /// (Counting a re-read as a change would re-read forever.)
    AnyChange,
    /// As `AnyChange`, and a file closed after a write too. For a watcher whose reader opens and
    /// closes every file it finds, where the close after a write is the only sign of an edit
    /// that made no other event.
    ClosedAfterWrite,
}

impl EventFilter {
    fn counts(self, event: &notify::Event) -> bool {
        match self {
            EventFilter::AnyChange => !event.kind.is_access(),
            EventFilter::ClosedAfterWrite => {
                matches!(
                    event.kind,
                    EventKind::Access(AccessKind::Close(AccessMode::Write))
                ) || !event.kind.is_access()
            }
        }
    }
}

/// Something to watch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Place {
    /// One file, watched through its folder: an editor's or an atomic save replaces the file, so
    /// the folder is what sees it. Its temp file and its siblings are not it.
    File(PathBuf),
    /// A folder and everything under it.
    Tree(PathBuf),
    /// A folder's own entries, not what is under them.
    Folder(PathBuf),
    /// A file that may not exist yet, nor its folders: the nearest folder above it that exists,
    /// and everything under that, so the file being created later is seen.
    Appearing(PathBuf),
}

impl Place {
    /// A folder as a [`Place::Tree`], anything else as a [`Place::File`].
    pub(crate) fn at(path: &Path) -> Place {
        if path.is_dir() {
            Place::Tree(path.to_path_buf())
        } else {
            Place::File(path.to_path_buf())
        }
    }

    /// The folder to watch for this place and whether to descend; `None` when there is none.
    fn folder(&self) -> Option<(PathBuf, RecursiveMode)> {
        match self {
            Place::File(file) => Some((file.parent()?.to_path_buf(), RecursiveMode::NonRecursive)),
            Place::Tree(dir) => Some((dir.clone(), RecursiveMode::Recursive)),
            Place::Folder(dir) => Some((dir.clone(), RecursiveMode::NonRecursive)),
            Place::Appearing(file) => {
                let above = file.parent()?.ancestors().find(|dir| dir.is_dir())?;
                Some((above.to_path_buf(), RecursiveMode::Recursive))
            }
        }
    }

    /// Whether a change at `path` is one this place cares about.
    fn covers(&self, path: &Path) -> bool {
        match self {
            Place::File(file) => {
                path.parent() == file.parent() && path.file_name() == file.file_name()
            }
            Place::Tree(dir) => path.starts_with(dir),
            Place::Folder(dir) => path.parent() == Some(dir.as_path()),
            Place::Appearing(file) => path == file,
        }
    }
}

/// A running watch; dropping it stops watching.
pub(crate) struct Debounced {
    signals: mpsc::UnboundedReceiver<()>,
    quiet: Duration,
    _watcher: RecommendedWatcher,
}

impl Debounced {
    /// Watch `places` for the events `filter` counts, reporting a burst settled once the places
    /// have been quiet for `quiet`. A place that cannot be watched is left out with a line on
    /// stderr; none that can be is an error. Needs a Tokio runtime (the watcher's callback sends
    /// into an unbounded channel; nothing here blocks it).
    pub(crate) fn start(
        places: Vec<Place>,
        filter: EventFilter,
        quiet: Duration,
    ) -> Result<Debounced, ServiceError> {
        let (signal, signals) = mpsc::unbounded_channel::<()>();
        let wanted = places.clone();
        let mut watcher =
            notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
                if let Ok(event) = res
                    && filter.counts(&event)
                    && event
                        .paths
                        .iter()
                        .any(|path| wanted.iter().any(|place| place.covers(path)))
                {
                    // A dropped signal means the watching task ended: nobody is left to re-read for.
                    let _ = signal.send(());
                }
            })?;
        let mut watched = 0_usize;
        for (folder, mode) in places.iter().filter_map(Place::folder) {
            match watcher.watch(&folder, mode) {
                Ok(()) => watched += 1,
                Err(error) => eprintln!("sill: watching {}: {error}", folder.display()),
            }
        }
        if watched == 0 {
            return Err(notify::Error::generic("no place could be watched").into());
        }
        Ok(Debounced {
            signals,
            quiet,
            _watcher: watcher,
        })
    }

    /// Watch the file `name` of `dir` (created if missing) for changes of the shell's own kind:
    /// [`OWN_FILE_QUIET`] of quiet, nothing but a real change.
    pub(crate) fn file(dir: &Path, name: &str) -> Result<Debounced, ServiceError> {
        std::fs::create_dir_all(dir).map_err(|source| ServiceError::Io {
            path: dir.to_path_buf(),
            source,
        })?;
        Debounced::start(
            vec![Place::File(dir.join(name))],
            EventFilter::AnyChange,
            OWN_FILE_QUIET,
        )
    }

    /// Wait for the next settled touch: the first signal, then every further signal within the
    /// quiet time restarts the wait. `None` once the watcher itself is gone.
    pub(crate) async fn settled(&mut self) -> Option<()> {
        self.signals.recv().await?;
        loop {
            match tokio::time::timeout(self.quiet, self.signals.recv()).await {
                Ok(Some(())) => continue,
                Ok(None) | Err(_) => return Some(()),
            }
        }
    }
}

/// [`Debounced::settled`] of a watch that may not exist: never, without one.
pub(crate) async fn settled(watch: &mut Option<Debounced>) -> Option<()> {
    match watch {
        Some(watch) => watch.settled().await,
        None => std::future::pending().await,
    }
}

#[cfg(test)]
mod tests {
    use notify::event::{
        AccessKind, AccessMode, CreateKind, DataChange, Event, EventKind, ModifyKind, RemoveKind,
    };
    use sill_testkit::TempDir;

    use super::{Debounced, EventFilter, OWN_FILE_QUIET, Place};

    const FILE_NAME: &str = "dock.json";

    /// A rename onto the file (the atomic writer's own move) settles as exactly one touch,
    /// and a change to an unrelated sibling file is not reported at all.
    #[tokio::test]
    async fn a_file_watch_fires_once_on_rename_and_ignores_siblings() {
        let dir = TempDir::new("fs-watch");
        let mut touched = Debounced::file(dir.path(), FILE_NAME).unwrap_or_else(|e| panic!("{e}"));

        let tmp = dir.path().join("dock.json.part");
        std::fs::write(&tmp, b"{}").unwrap_or_else(|e| panic!("{e}"));
        std::fs::rename(&tmp, dir.path().join(FILE_NAME)).unwrap_or_else(|e| panic!("{e}"));

        let settled = tokio::time::timeout(std::time::Duration::from_secs(2), touched.settled())
            .await
            .unwrap_or_else(|_| panic!("no touch reported within 2s"));
        assert_eq!(settled, Some(()));

        std::fs::write(dir.path().join("unrelated"), b"x").unwrap_or_else(|e| panic!("{e}"));
        let extra = tokio::time::timeout(OWN_FILE_QUIET * 4, touched.settled()).await;
        assert!(extra.is_err(), "a sibling file must not be reported");
    }

    /// A scan's own reads (open, read, close without writing) must not count as a change, or the
    /// watcher feeds the scan that feeds it.
    #[test]
    fn reads_are_never_changes_and_a_close_after_a_write_counts_only_where_asked() {
        let kinds = [
            (
                EventKind::Access(AccessKind::Open(AccessMode::Any)),
                false,
                false,
            ),
            (EventKind::Access(AccessKind::Read), false, false),
            (
                EventKind::Access(AccessKind::Close(AccessMode::Read)),
                false,
                false,
            ),
            (
                EventKind::Access(AccessKind::Close(AccessMode::Write)),
                false,
                true,
            ),
            (EventKind::Create(CreateKind::File), true, true),
            (
                EventKind::Modify(ModifyKind::Data(DataChange::Any)),
                true,
                true,
            ),
            (EventKind::Remove(RemoveKind::File), true, true),
        ];
        for (kind, any, after_write) in kinds {
            let event = Event::new(kind);
            assert_eq!(EventFilter::AnyChange.counts(&event), any, "{kind:?}");
            assert_eq!(
                EventFilter::ClosedAfterWrite.counts(&event),
                after_write,
                "{kind:?}"
            );
        }
    }

    #[test]
    fn a_place_covers_its_file_its_tree_or_the_one_path_it_waits_for() {
        use std::path::Path;
        let file = Place::File("/d/dock.json".into());
        assert!(file.covers(Path::new("/d/dock.json")));
        assert!(!file.covers(Path::new("/d/dock.json.part")));
        assert!(!file.covers(Path::new("/d/sub/dock.json")));
        let tree = Place::Tree("/d".into());
        assert!(tree.covers(Path::new("/d/a/b.ics")));
        assert!(!tree.covers(Path::new("/e/b.ics")));
        let later = Place::Appearing("/d/sill/wallpaper-source".into());
        assert!(later.covers(Path::new("/d/sill/wallpaper-source")));
        assert!(!later.covers(Path::new("/d/sill")));
    }

    #[test]
    fn an_appearing_file_is_watched_from_the_nearest_folder_that_exists() {
        let dir = TempDir::new("debounced-appearing");
        let target = dir.path().join("sill/wallpaper-source");
        let above = |place: &Place| place.folder().map(|(folder, _)| folder);
        assert_eq!(
            above(&Place::Appearing(target.clone())),
            Some(dir.path().to_path_buf())
        );
        std::fs::create_dir_all(dir.path().join("sill")).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(
            above(&Place::Appearing(target)),
            Some(dir.path().join("sill"))
        );
    }
}
