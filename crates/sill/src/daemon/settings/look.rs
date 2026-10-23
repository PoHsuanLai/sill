//! The look: the third sheet of the cascade, from the look Extension `appearance.look` names.
//! [`LookSearch`] holds the data directories Extensions are installed in; [`chosen_look`] turns
//! the choice into the sheet and the folder its pictures may come from, and [`spawn_look_watch`]
//! tells the host thread when `extensions.toml` or an Extension folder changed, so the keeper
//! looks again.
//!
//! Nothing here fails: a look that is off, gone, damaged or too big is logged on stderr, one line,
//! and the desktop then looks as it ships (`standard`).

use std::path::PathBuf;
use std::time::Duration;

use ds::style::kit::LookStyle;
use ds_blitz::TokioSpawner;
use ds_extensions::{Capability, ExtensionsFile, Search, look};
use ds_settings::{AppearanceFile, Store, WatchState};
use notify::{RecursiveMode, Watcher};
use shell_host::HostHandle;
use sill_services::env::Dirs;
use tokio::runtime::Runtime;
use tokio::sync::mpsc;

use super::store::report;
use crate::daemon::msg::ShellMsg;

/// How long a burst of file events settles before the look is looked for again.
const SETTLE: Duration = Duration::from_millis(150);

/// Where Extensions are installed: `<data>/quire/extensions/<id>/` in the person's data directory
/// and in each system one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LookSearch {
    person: PathBuf,
    system: Vec<PathBuf>,
}

impl LookSearch {
    /// The search of `person`'s data directory and each of the `system` ones, most important first.
    pub fn new(person: PathBuf, system: Vec<PathBuf>) -> LookSearch {
        LookSearch { person, system }
    }

    /// The search over the data directories `dirs` resolved.
    pub fn of(dirs: &Dirs) -> LookSearch {
        LookSearch::new(dirs.data.clone(), dirs.data_dirs.clone())
    }

    fn search(&self) -> Search {
        Search::new(&self.person, &self.system)
    }

    /// Every folder Extensions may be in, the person's first.
    fn folders(&self) -> Vec<PathBuf> {
        std::iter::once(&self.person)
            .chain(&self.system)
            .map(|root| root.join("quire").join("extensions"))
            .collect()
    }
}

/// The chosen look's sheet and the folder its `url()` files may come from. Both are empty for
/// `standard` and for a look that cannot be used.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ChosenLook {
    /// The sheet's text.
    pub style: LookStyle,
    /// The Extension's folder.
    pub home: Option<PathBuf>,
}

/// The look `appearance` names, found among the Extensions `search` sees that `extensions`
/// (`quire/extensions.toml`) has not turned off.
pub fn chosen_look(extensions: &Store, appearance: &Store, search: &LookSearch) -> ChosenLook {
    let choice = appearance.load::<AppearanceFile>().value.appearance.look;
    let Some(id) = choice.chosen() else {
        return ChosenLook::default();
    };
    let off = extensions.load::<ExtensionsFile>().value.off_ids();
    let resolved = search.search().scan(&off).resolve(Capability::Look);
    for problem in resolved.problems().iter().filter(|p| p.id.0 == id) {
        eprintln!("sill: the look {id}: {}", problem.words());
    }
    match look::stylesheet_for(&choice, &resolved.registry) {
        Some(sheet) => ChosenLook {
            style: LookStyle(sheet.text),
            home: Some(sheet.folder),
        },
        None => {
            eprintln!("sill: the look {id} is not available; using the standard look");
            ChosenLook::default()
        }
    }
}

/// Watch `extensions`' file and the Extension folders on `runtime`, sending `ShellMsg::Look` to
/// `host` after each settled change. A place that cannot be watched is logged and the look then
/// changes only through `sill reload` and `appearance.look`.
pub fn spawn_look_watch(
    runtime: &Runtime,
    extensions: &Store,
    search: &LookSearch,
    host: HostHandle<ShellMsg>,
) {
    let mut file = extensions.watch::<ExtensionsFile>(&TokioSpawner::on(runtime.handle().clone()));
    if let WatchState::Blind { reason } = file.state() {
        eprintln!("sill: watching the extensions list: {reason}");
    }
    let (touched, mut touches) = mpsc::unbounded_channel();
    let watcher = watch_folders(search, touched);
    runtime.spawn(async move {
        // Dropping the watcher ends the folder watch; it lives as long as this task.
        let _folders = watcher;
        loop {
            tokio::select! {
                Some(loaded) = file.changed() => report(&loaded),
                Some(()) = touches.recv() => {
                    tokio::time::sleep(SETTLE).await;
                    while touches.try_recv().is_ok() {}
                }
                else => break,
            }
            if host.send(ShellMsg::Look).is_err() {
                break;
            }
        }
    });
}

/// A watch on each Extension folder that exists (the person's is made), telling `touched` of each
/// change that is not just a file being read. `None` when nothing could be watched.
fn watch_folders(
    search: &LookSearch,
    touched: mpsc::UnboundedSender<()>,
) -> Option<notify::RecommendedWatcher> {
    let mut watcher = notify::recommended_watcher(move |event: notify::Result<notify::Event>| {
        if event.is_ok_and(|event| !event.kind.is_access()) {
            // The receiver is gone once the watch task ended; nobody is left to tell.
            let _ = touched.send(());
        }
    })
    .inspect_err(|error| eprintln!("sill: watching the extensions: {error}"))
    .ok()?;
    let folders = search.folders();
    if let Some(own) = folders.first() {
        // Where an installed Extension goes; made so the first one is seen without a restart.
        let _best_effort = std::fs::create_dir_all(own);
    }
    for folder in folders.iter().filter(|folder| folder.is_dir()) {
        if let Err(error) = watcher.watch(folder, RecursiveMode::Recursive) {
            eprintln!("sill: watching {}: {error}", folder.display());
        }
    }
    Some(watcher)
}

#[cfg(test)]
#[path = "look_tests.rs"]
pub(super) mod tests;
