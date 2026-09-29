//! Watching COSMIC's background config for edits: `notify` on the nearest directory that
//! exists, 500 ms debounce (design/20 §1.4's own cache row names Lanczos3 and the cross-fade).
//! Unlike `sill-settings::watch` (30 ms, a file `sill` itself always creates), this file may not
//! exist at all outside a COSMIC session, so the watch starts on whichever ancestor of it is
//! actually there and widens to catch the file being created later.

use std::path::{Path, PathBuf};
use std::time::Duration;

use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use tokio::runtime::Handle;
use tokio::sync::mpsc;

/// The config watch's debounce (distinct from `sill-settings`'s 30 ms: a foreign config file is
/// edited far less often than the shell's own, and cosmic-bg can touch it more than once per
/// background change).
pub const DEBOUNCE: Duration = Duration::from_millis(500);

/// The nearest existing ancestor of `path` — `path` itself excluded, since a watch's purpose
/// here is to notice `path` being created. `None` when nothing above it exists either (no
/// filesystem root at all).
fn nearest_existing_ancestor(path: &Path) -> Option<PathBuf> {
    path.parent()?
        .ancestors()
        .find(|candidate| candidate.is_dir())
        .map(Path::to_path_buf)
}

/// Whether `event` touches `target` (COSMIC's `all` record): not an `Access` (a re-read is not
/// a change), and one of its paths is exactly `target`.
fn touches(event: &notify::Event, target: &Path) -> bool {
    !event.kind.is_access() && event.paths.iter().any(|path| path == target)
}

/// Start watching for `target` to change or appear, debouncing onto `rt` (never the bare
/// `tokio::spawn`: the caller may not have entered a runtime on its own thread — `services.rs`'s
/// own test calls `Services::spawn` from a plain thread holding only a `Handle`, and the real
/// daemon spawns every service before it ever enters one; only `Handle::spawn` works from
/// there). `None` when no ancestor of it exists at all; the caller still has
/// `config::resolve_source`'s read at spawn time, it just never updates live.
pub fn watch(
    target: &Path,
    rt: &Handle,
) -> Option<(RecommendedWatcher, mpsc::UnboundedReceiver<()>)> {
    let root = nearest_existing_ancestor(target)?;
    let (raw_tx, mut raw_rx) = mpsc::unbounded_channel::<()>();
    let owned_target = target.to_path_buf();
    let mut watcher = notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
        if let Ok(event) = res
            && touches(&event, &owned_target)
        {
            let _ = raw_tx.send(());
        }
    })
    .ok()?;
    // The file's own parent may not exist yet (COSMIC writes it lazily); recursing from the
    // nearest ancestor that does catches both "the file changed" and "the file was created".
    watcher.watch(&root, RecursiveMode::Recursive).ok()?;
    let (settled_tx, settled_rx) = mpsc::unbounded_channel::<()>();
    rt.spawn(async move {
        while raw_rx.recv().await.is_some() {
            loop {
                match tokio::time::timeout(DEBOUNCE, raw_rx.recv()).await {
                    Ok(Some(())) => continue,
                    Ok(None) => return,
                    Err(_settled) => break,
                }
            }
            if settled_tx.send(()).is_err() {
                return;
            }
        }
    });
    Some((watcher, settled_rx))
}

#[cfg(test)]
mod tests {
    use super::nearest_existing_ancestor;

    #[test]
    fn the_nearest_existing_ancestor_skips_missing_directories() {
        let dir = std::env::temp_dir().join(format!("sill-wallpaper-watch-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap_or_else(|e| panic!("{e}"));
        let target = dir.join("cosmic/com.system76.CosmicBackground/v1/all");
        assert_eq!(nearest_existing_ancestor(&target), Some(dir.clone()));

        std::fs::create_dir_all(dir.join("cosmic/com.system76.CosmicBackground/v1"))
            .unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(
            nearest_existing_ancestor(&target),
            Some(dir.join("cosmic/com.system76.CosmicBackground/v1"))
        );

        let _ = std::fs::remove_dir_all(&dir);
    }
}
