//! The folders' watch: [`crate::debounced`] on each folder that exists (not recursive); once a burst
//! of writes settles, the folders are listed and each screenshot that is new, or written again,
//! becomes `ShotEvent::Appeared`. A partial file (`.part`, a hidden temp name) is nothing until it
//! is renamed to its final name.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use sill_model::screenshots::{ShotEvent, ShotName, Watching, is_screenshot_name};
use tokio::sync::mpsc;
use tokio::task::JoinSet;

use super::live::watching_of;
use crate::debounced::{Debounced, EventFilter, Place};

/// How long the folders must be quiet before a burst of writes is looked at: a tool that writes
/// a shot in several steps, or saves two, gives one update.
const SETTLE: Duration = Duration::from_millis(50);

/// The screenshots in a folder and when each was last written.
type Shots = BTreeMap<PathBuf, SystemTime>;

/// Watch the folders of `folders` that exist, sending each screenshot that lands in one. The
/// watch lives in the returned set of tasks and ends when it is dropped.
pub(crate) fn start(
    folders: &[PathBuf],
    events: mpsc::UnboundedSender<ShotEvent>,
) -> (JoinSet<()>, Watching) {
    let existing: Vec<PathBuf> = folders.iter().filter(|f| f.is_dir()).cloned().collect();
    if existing.is_empty() {
        return (JoinSet::new(), Watching::Off);
    }
    let places = existing.iter().cloned().map(Place::Folder).collect();
    let mut touches = match Debounced::start(places, EventFilter::ClosedAfterWrite, SETTLE) {
        Ok(touches) => touches,
        Err(error) => return (JoinSet::new(), Watching::Failed(error.to_string())),
    };
    let mut known = shots_in(&existing);
    let mut task = JoinSet::new();
    let looked_in = existing.clone();
    task.spawn(async move {
        while touches.settled().await.is_some() {
            let scan = looked_in.clone();
            let Ok(now) = tokio::task::spawn_blocking(move || shots_in(&scan)).await else {
                return;
            };
            for (path, written) in &now {
                if known.get(path) != Some(written)
                    && events.send(ShotEvent::Appeared(path.clone())).is_err()
                {
                    return;
                }
            }
            known = now;
        }
    });
    (task, watching_of(existing))
}

/// The files in `folders` whose names read as finished screenshots. A partial file (`.part`, a
/// hidden temp name) is nothing until it is renamed to its final name.
fn shots_in(folders: &[PathBuf]) -> Shots {
    folders
        .iter()
        .filter_map(|folder| std::fs::read_dir(folder).ok())
        .flatten()
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| named(&path.as_path()))
        .filter_map(|path| {
            let written = path.metadata().ok()?.modified().ok()?;
            Some((path, written))
        })
        .collect()
}

/// Whether `path`'s file name reads as a finished screenshot.
fn named(path: &&Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| is_screenshot_name(name) == ShotName::Screenshot)
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use sill_testkit::TempDir;

    use super::{named, shots_in};

    #[test]
    fn only_a_screenshots_name_counts() {
        assert!(named(&Path::new("/s/Screenshot_1.png")));
        assert!(!named(&Path::new("/s/Screenshot_1.png.part")));
        assert!(!named(&Path::new("/s/.Screenshot_1.png")));
    }

    #[test]
    fn a_folder_lists_its_finished_screenshots_only() {
        let dir = TempDir::new("shots-in");
        for name in ["Screenshot_1.png", "Screenshot_2.png.part", "notes.txt"] {
            std::fs::write(dir.path().join(name), b"x").unwrap_or_else(|e| panic!("{e}"));
        }
        let found: Vec<_> = shots_in(&[dir.path().to_path_buf()]).into_keys().collect();
        assert_eq!(found, [dir.path().join("Screenshot_1.png")]);
    }
}
