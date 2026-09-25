//! The folders' watch: `notify` on each folder that exists (not recursive), turning a file
//! closed after writing, or renamed into place, whose name reads as a screenshot, into
//! `ShotEvent::Appeared`. A partial file (`.part`, a hidden temp name) is nothing until it is
//! renamed to its final name.

use std::path::{Path, PathBuf};

use notify::event::{AccessKind, AccessMode, ModifyKind, RenameMode};
use notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use tokio::sync::mpsc;

use super::live::watching_of;
use super::names::{ShotName, is_screenshot_name};
use super::reduce::{ShotEvent, Watching};

/// Watch the folders of `folders` that exist, sending each screenshot that lands in one.
pub(super) fn start(
    folders: &[PathBuf],
    events: mpsc::UnboundedSender<ShotEvent>,
) -> (Option<RecommendedWatcher>, Watching) {
    let existing: Vec<PathBuf> = folders.iter().filter(|f| f.is_dir()).cloned().collect();
    if existing.is_empty() {
        return (None, Watching::Off);
    }
    let made = notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
        if let Some(path) = res.ok().as_ref().and_then(landed).filter(named) {
            let _ = events.send(ShotEvent::Appeared(path.to_path_buf()));
        }
    });
    let mut watcher = match made {
        Ok(watcher) => watcher,
        Err(error) => return (None, Watching::Failed(error.to_string())),
    };
    for folder in &existing {
        if let Err(error) = watcher.watch(folder, RecursiveMode::NonRecursive) {
            return (
                None,
                Watching::Failed(format!("{}: {error}", folder.display())),
            );
        }
    }
    (Some(watcher), watching_of(existing))
}

/// The file an event says has just landed: closed after writing, or renamed into place.
fn landed(event: &notify::Event) -> Option<&Path> {
    let path = match event.kind {
        EventKind::Access(AccessKind::Close(AccessMode::Write))
        | EventKind::Modify(ModifyKind::Name(RenameMode::To)) => event.paths.first(),
        EventKind::Modify(ModifyKind::Name(RenameMode::Both)) => event.paths.last(),
        _ => None,
    };
    path.map(PathBuf::as_path)
}

/// Whether `path`'s file name reads as a finished screenshot.
fn named(path: &&Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| is_screenshot_name(name) == ShotName::Screenshot)
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use notify::event::{AccessKind, AccessMode, CreateKind, ModifyKind, RenameMode};
    use notify::{Event, EventKind};

    use super::{landed, named};

    #[test]
    fn a_close_after_writing_or_a_rename_into_place_lands() {
        let a = PathBuf::from("/s/.tmp-1");
        let b = PathBuf::from("/s/Screenshot_1.png");
        let event = |kind, paths: &[&PathBuf]| Event {
            kind,
            paths: paths.iter().map(|p| (*p).clone()).collect(),
            attrs: Default::default(),
        };
        let close = EventKind::Access(AccessKind::Close(AccessMode::Write));
        let to = EventKind::Modify(ModifyKind::Name(RenameMode::To));
        let both = EventKind::Modify(ModifyKind::Name(RenameMode::Both));
        let from = EventKind::Modify(ModifyKind::Name(RenameMode::From));
        let create = EventKind::Create(CreateKind::File);
        let cases = [
            (event(close, &[&b]), Some(b.as_path())),
            (event(to, &[&b]), Some(b.as_path())),
            (event(both, &[&a, &b]), Some(b.as_path())),
            (event(from, &[&a]), None),
            (event(create, &[&b]), None),
        ];
        for (event, want) in cases {
            assert_eq!(landed(&event), want, "{:?}", event.kind);
        }
    }

    #[test]
    fn only_a_screenshots_name_counts() {
        assert!(named(&Path::new("/s/Screenshot_1.png")));
        assert!(!named(&Path::new("/s/Screenshot_1.png.part")));
        assert!(!named(&Path::new("/s/.Screenshot_1.png")));
    }
}
