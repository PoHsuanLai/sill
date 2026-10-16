//! Cover art: a local regular file as it is, a remote one fetched once into the cache within a
//! time, a size and a number of fetches at once ([`ArtFetcher`]).

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use sill_model::now_playing::{
    Art, ArtSource, Media, MediaEvent, PlayerId, art_cache_name, art_source,
};
use tokio::sync::{Semaphore, mpsc, watch};
use tokio::task::JoinHandle;
use zbus::zvariant::OwnedValue;

use super::run::{apply, metadata_string};

/// Whether `path` is a regular file no larger than [`ART_MAX_BYTES`].
fn is_cover_file(path: &Path) -> bool {
    std::fs::metadata(path).is_ok_and(|meta| meta.is_file() && meta.len() <= ART_MAX_BYTES)
}

/// The art `metadata` names, resolved as far as a bus read can take it: a local regular file of a
/// sane size is used as it is (a device, a pipe or a huge file is no art), a remote one already in
/// the cache is used from there, and a remote one not yet cached is a job for
/// [`ArtFetcher::spawn`].
pub(crate) fn resolve_art(
    metadata: &HashMap<String, OwnedValue>,
    cache_dir: Option<&Path>,
) -> (Art, Option<(String, PathBuf)>) {
    let url = metadata_string(metadata, "mpris:artUrl");
    match art_source(&url) {
        ArtSource::File(path) if is_cover_file(&path) => (Art::File(path), None),
        ArtSource::File(_) => (Art::None, None),
        ArtSource::Remote(url) => match cache_dir {
            Some(dir) => {
                let cache_path = dir.join(art_cache_name(&url));
                if cache_path.is_file() {
                    (Art::File(cache_path), None)
                } else {
                    (Art::None, Some((url, cache_path)))
                }
            }
            None => (Art::None, None),
        },
        ArtSource::Unusable => (Art::None, None),
    }
}

/// The largest cover a fetch may bring: a cover is drawn a few hundred pixels across.
const ART_MAX_BYTES: u64 = 5 * 1024 * 1024;

/// How long one fetch may take, connecting included.
const FETCH_TIME_LIMIT: Duration = Duration::from_secs(20);

/// How many fetches run at once, whatever the players publish.
const FETCHES_AT_ONCE: usize = 2;

/// How many redirects one fetch follows.
const REDIRECTS_MAX: &str = "5";

/// How many covers the cache keeps: the oldest go first.
const CACHE_FILES_MAX: usize = 64;

/// The art fetches: a download per player at a time, a few at once, each cut off by a time and a
/// size, killed when dropped, kept to the cache's size. A player that publishes a new cover every
/// second replaces its own fetch; it never adds to the others.
#[derive(Debug)]
pub(crate) struct ArtFetcher {
    done: mpsc::UnboundedSender<(PlayerId, PathBuf)>,
    slots: Arc<Semaphore>,
    running: Mutex<HashMap<PlayerId, Fetch>>,
}

/// A fetch under way for one player.
#[derive(Debug)]
struct Fetch {
    cache_path: PathBuf,
    task: JoinHandle<()>,
}

impl ArtFetcher {
    /// A fetcher and the receiver of the paths it finishes.
    pub(crate) fn new() -> (ArtFetcher, mpsc::UnboundedReceiver<(PlayerId, PathBuf)>) {
        let (done, finished) = mpsc::unbounded_channel();
        let fetcher = ArtFetcher {
            done,
            slots: Arc::new(Semaphore::new(FETCHES_AT_ONCE)),
            running: Mutex::new(HashMap::new()),
        };
        (fetcher, finished)
    }

    /// Download `url` into `cache_path` with `curl` (a runtime requirement of the desktop; without
    /// it no cover art is fetched) and report the path on success. [`resolve_art`] only
    /// schedules this for art that is not already cached. A fetch already running for the same
    /// file is left alone; one for another cover of the same player is stopped.
    pub(crate) fn spawn(&self, id: PlayerId, url: String, cache_path: PathBuf) {
        let mut running = self
            .running
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        running.retain(|_, fetch| !fetch.task.is_finished());
        if running
            .get(&id)
            .is_some_and(|fetch| fetch.cache_path == cache_path)
        {
            return;
        }
        let (done, slots) = (self.done.clone(), Arc::clone(&self.slots));
        let (player, target) = (id.clone(), cache_path.clone());
        let task = tokio::spawn(async move {
            let Ok(_slot) = slots.acquire().await else {
                return;
            };
            if fetch(&url, &target).await {
                let _ = done.send((player, target));
            }
        });
        if let Some(stopped) = running.insert(id, Fetch { cache_path, task }) {
            stopped.task.abort();
        }
    }
}

impl Drop for ArtFetcher {
    fn drop(&mut self) {
        let running = self
            .running
            .get_mut()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        for fetch in running.values() {
            fetch.task.abort();
        }
    }
}

/// Whether `url` was brought to `cache_path`: fetched to a part file within the time and size
/// limits (http and https only, also across redirects, no globbing of the address), then renamed
/// into place, so a stopped fetch never leaves a half cover that looks cached.
async fn fetch(url: &str, cache_path: &Path) -> bool {
    let Some(dir) = cache_path.parent() else {
        return false;
    };
    let _ = tokio::fs::create_dir_all(dir).await;
    prune_cache(dir.to_owned()).await;
    let mut part = cache_path.as_os_str().to_owned();
    part.push(".part");
    let part = PathBuf::from(part);
    let (max_time, max_size) = (
        FETCH_TIME_LIMIT.as_secs().to_string(),
        ART_MAX_BYTES.to_string(),
    );
    // curl's own limit is the first line; this one covers a curl that does not return.
    let limit = FETCH_TIME_LIMIT + Duration::from_secs(5);
    let status = tokio::time::timeout(
        limit,
        tokio::process::Command::new("curl")
            .args(["--silent", "--fail", "--location", "--globoff"])
            .args(["--proto", "=http,https", "--proto-redir", "=http,https"])
            .args(["--max-redirs", REDIRECTS_MAX])
            .args(["--max-time", max_time.as_str()])
            .args(["--max-filesize", max_size.as_str()])
            .arg("--output")
            .arg(&part)
            .arg("--")
            .arg(url)
            .kill_on_drop(true)
            .status(),
    )
    .await;
    let fetched = matches!(status, Ok(Ok(status)) if status.success()) && within_size(&part).await;
    if fetched && tokio::fs::rename(&part, cache_path).await.is_ok() {
        return true;
    }
    let _ = tokio::fs::remove_file(&part).await;
    false
}

/// Whether the file at `path` is no larger than [`ART_MAX_BYTES`] (curl's own limit cannot see a
/// reply of unknown length).
async fn within_size(path: &Path) -> bool {
    tokio::fs::metadata(path)
        .await
        .is_ok_and(|meta| meta.is_file() && meta.len() <= ART_MAX_BYTES)
}

/// The cache directory held to [`CACHE_FILES_MAX`] covers: the oldest files go first.
async fn prune_cache(dir: PathBuf) {
    let _ = tokio::task::spawn_blocking(move || {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            return;
        };
        let mut files: Vec<(std::time::SystemTime, PathBuf)> = entries
            .filter_map(Result::ok)
            .filter_map(|entry| {
                let meta = entry.metadata().ok()?;
                let modified = meta.modified().ok()?;
                meta.is_file().then(|| (modified, entry.path()))
            })
            .collect();
        files.sort();
        let excess = files.len().saturating_sub(CACHE_FILES_MAX);
        for (_, path) in files.into_iter().take(excess) {
            let _ = std::fs::remove_file(path);
        }
    })
    .await;
}

/// A download finished: folds the cached path into whatever that player's track looks like now.
pub(crate) fn apply_art(tx: &watch::Sender<Media>, id: PlayerId, path: PathBuf) {
    let media = tx.borrow().clone();
    let Some(player) = media.players.iter().find(|p| p.id == id) else {
        return;
    };
    let mut track = player.track.clone();
    track.art = Art::File(path);
    apply(tx, MediaEvent::Track { id, track });
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{ART_MAX_BYTES, CACHE_FILES_MAX, is_cover_file, prune_cache};

    fn scratch(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("sill-art-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap_or_else(|e| panic!("{e}"));
        dir
    }

    #[test]
    fn only_a_regular_file_of_a_sane_size_is_a_cover() {
        let dir = scratch("cover");
        let small = dir.join("small.png");
        std::fs::write(&small, b"png").unwrap_or_else(|e| panic!("{e}"));
        let big = dir.join("big.png");
        std::fs::File::create(&big)
            .and_then(|file| file.set_len(ART_MAX_BYTES + 1))
            .unwrap_or_else(|e| panic!("{e}"));
        assert!(is_cover_file(&small));
        assert!(!is_cover_file(&big), "past the ceiling");
        assert!(!is_cover_file(&dir), "a directory");
        assert!(!is_cover_file(&dir.join("missing.png")));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn the_cache_keeps_its_newest_covers() {
        let dir = scratch("prune");
        let total = CACHE_FILES_MAX + 10;
        for n in 0..total {
            let path = dir.join(format!("{n:04}.png"));
            std::fs::write(&path, b"x").unwrap_or_else(|e| panic!("{e}"));
            let age = std::time::Duration::from_secs((total - n) as u64 * 10);
            let when = std::time::SystemTime::now() - age;
            std::fs::File::options()
                .write(true)
                .open(&path)
                .and_then(|file| file.set_modified(when))
                .unwrap_or_else(|e| panic!("{e}"));
        }
        prune_cache(dir.clone()).await;
        let left = std::fs::read_dir(&dir).map_or(0, Iterator::count);
        assert_eq!(left, CACHE_FILES_MAX);
        assert!(!dir.join("0000.png").exists(), "the oldest went");
        assert!(
            dir.join(format!("{:04}.png", total - 1)).exists(),
            "the newest stayed"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
