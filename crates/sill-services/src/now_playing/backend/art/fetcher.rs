//! The cover jobs: one at a time per player (a newer cover replaces an older one that has not
//! finished), one per cover however many players name it, a gap between one player's jobs, and a
//! few at once overall. Each is cut off by a time and a size ([`super::curl`], [`super::cache`]).

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError, Weak};
use std::time::{Duration, Instant};

use sill_model::now_playing::{PlayerId, next_start};
use tokio::sync::{Semaphore, mpsc};
use tokio::task::JoinHandle;

use super::{ArtJob, cache, curl};

/// How many jobs run at once, whatever the players publish.
const JOBS_AT_ONCE: usize = 2;

/// The gap between the starts of one player's jobs: a real player changes cover once a track, so
/// a player that changes faster than this is hostile or stuck, and only its newest cover is
/// brought.
const PLAYER_GAP: Duration = Duration::from_secs(3);

/// The cover jobs, started by [`ArtFetcher::spawn`], killed when dropped.
#[derive(Debug)]
pub(crate) struct ArtFetcher {
    done: mpsc::UnboundedSender<(PlayerId, PathBuf)>,
    slots: Arc<Semaphore>,
    turns: Arc<Turns>,
    running: Mutex<HashMap<PlayerId, Scheduled>>,
}

/// A job under way, or waiting for its turn, for one player.
#[derive(Debug)]
struct Scheduled {
    cache_path: PathBuf,
    starts: Instant,
    task: JoinHandle<()>,
}

/// One lock per cover that has a job: a second job for the same cover waits for the first and
/// then finds the file. A lock goes when no job holds or waits for it.
#[derive(Debug, Default)]
struct Turns(Mutex<HashMap<PathBuf, Weak<tokio::sync::Mutex<()>>>>);

impl Turns {
    fn of(&self, cache_path: &Path) -> Arc<tokio::sync::Mutex<()>> {
        let mut turns = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(turn) = turns.get(cache_path).and_then(Weak::upgrade) {
            return turn;
        }
        turns.retain(|_, turn| turn.strong_count() > 0);
        let turn = Arc::new(tokio::sync::Mutex::new(()));
        turns.insert(cache_path.to_owned(), Arc::downgrade(&turn));
        turn
    }
}

/// Whether `cache_path` holds its cover: it already does, or `work` brings it, run once however
/// many jobs name the cover and within `slots`. The lock for the cover is taken before a slot, so
/// a job waiting for another's cover never holds a slot idle.
async fn bring_once(
    turns: &Turns,
    slots: &Semaphore,
    cache_path: &Path,
    work: impl Future<Output = bool>,
) -> bool {
    let turn = turns.of(cache_path);
    let _only = turn.lock().await;
    if cache_path.is_file() {
        return true;
    }
    let Ok(_slot) = slots.acquire().await else {
        return false;
    };
    work.await
}

impl ArtFetcher {
    /// A fetcher and the receiver of the paths it finishes.
    pub(crate) fn new() -> (ArtFetcher, mpsc::UnboundedReceiver<(PlayerId, PathBuf)>) {
        let (done, finished) = mpsc::unbounded_channel();
        let fetcher = ArtFetcher {
            done,
            slots: Arc::new(Semaphore::new(JOBS_AT_ONCE)),
            turns: Arc::new(Turns::default()),
            running: Mutex::new(HashMap::new()),
        };
        (fetcher, finished)
    }

    /// Bring `job`'s cover into the cache and report the path on success. [`super::resolve_art`]
    /// only schedules this for a cover that is not already cached. A job already running for the
    /// same cover and player is left alone; one for another cover of the same player is replaced,
    /// and starts no sooner than [`PLAYER_GAP`] after that player's last one.
    pub(crate) fn spawn(&self, id: PlayerId, job: ArtJob) {
        let now = Instant::now();
        let mut running = self.running.lock().unwrap_or_else(PoisonError::into_inner);
        // Kept past their end for the gap they set.
        running.retain(|_, job| !job.task.is_finished() || job.starts + PLAYER_GAP > now);
        let cache_path = job.cache_path().to_owned();
        if running
            .get(&id)
            .is_some_and(|job| job.cache_path == cache_path && !job.task.is_finished())
        {
            return;
        }
        let starts = next_start(running.get(&id).map(|job| job.starts), now, PLAYER_GAP);
        let (done, slots, turns) = (
            self.done.clone(),
            Arc::clone(&self.slots),
            Arc::clone(&self.turns),
        );
        let (player, path) = (id.clone(), cache_path.clone());
        let task = tokio::spawn(async move {
            tokio::time::sleep_until(tokio::time::Instant::from_std(starts)).await;
            if bring_once(&turns, &slots, &path, carry_out(&job)).await {
                prune_after(&path).await;
                let _ = done.send((player, path));
            }
        });
        let scheduled = Scheduled {
            cache_path,
            starts,
            task,
        };
        if let Some(replaced) = running.insert(id, scheduled) {
            replaced.task.abort();
        }
    }
}

impl Drop for ArtFetcher {
    fn drop(&mut self) {
        let running = self
            .running
            .get_mut()
            .unwrap_or_else(PoisonError::into_inner);
        for job in running.values() {
            job.task.abort();
        }
    }
}

/// Run `job`.
async fn carry_out(job: &ArtJob) -> bool {
    match job {
        ArtJob::Fetch { url, cache_path } => curl::fetch(url, cache_path).await,
        ArtJob::Copy { from, cache_path } => {
            let (from, to) = (from.clone(), cache_path.clone());
            tokio::task::spawn_blocking(move || cache::copy_cover(&from, &to).is_ok())
                .await
                .unwrap_or(false)
        }
    }
}

/// Hold the folder `cover` is in to the cache's limits, now that it has one more file.
async fn prune_after(cover: &Path) {
    if let Some(dir) = cover.parent() {
        cache::prune(dir.to_owned()).await;
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use sill_testkit::TempDir;
    use tokio::sync::Semaphore;

    use super::{Turns, bring_once};

    /// A job that counts its runs and leaves the cover where it was asked to.
    async fn make(runs: &AtomicUsize, at: &Path) -> bool {
        runs.fetch_add(1, Ordering::SeqCst);
        // Long enough for the other job to be waiting for its turn.
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        std::fs::write(at, b"cover").is_ok()
    }

    #[tokio::test]
    async fn two_jobs_for_one_cover_fetch_it_once() {
        let dir = TempDir::new("art-once");
        let (turns, slots) = (Turns::default(), Semaphore::new(2));
        let (runs, cover) = (AtomicUsize::new(0), dir.path().join("a.png"));
        let (first, second) = tokio::join!(
            bring_once(&turns, &slots, &cover, make(&runs, &cover)),
            bring_once(&turns, &slots, &cover, make(&runs, &cover)),
        );
        assert!(first && second, "both players get the cover");
        assert_eq!(runs.load(Ordering::SeqCst), 1, "one fetch for one cover");
    }

    #[tokio::test]
    async fn jobs_for_different_covers_each_run() {
        let dir = TempDir::new("art-each");
        let (turns, slots) = (Turns::default(), Semaphore::new(2));
        let runs = AtomicUsize::new(0);
        let (a, b) = (dir.path().join("a.png"), dir.path().join("b.png"));
        let (first, second) = tokio::join!(
            bring_once(&turns, &slots, &a, make(&runs, &a)),
            bring_once(&turns, &slots, &b, make(&runs, &b)),
        );
        assert!(first && second);
        assert_eq!(runs.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn a_cover_already_in_the_cache_runs_nothing() {
        let dir = TempDir::new("art-cached");
        let (turns, slots) = (Turns::default(), Semaphore::new(1));
        let (runs, cover) = (AtomicUsize::new(0), dir.path().join("a.png"));
        std::fs::write(&cover, b"cover").unwrap_or_else(|e| panic!("{e}"));
        assert!(bring_once(&turns, &slots, &cover, make(&runs, &cover)).await);
        assert_eq!(runs.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn a_lock_is_shared_while_a_job_holds_it_and_gone_after() {
        let turns = Turns::default();
        let path = Path::new("/c/a.png");
        let held = turns.of(path);
        assert!(std::sync::Arc::ptr_eq(&held, &turns.of(path)), "shared");
        drop(held);
        let fresh = turns.of(path);
        let map = turns
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        assert_eq!(map.len(), 1, "the dead lock was replaced, not kept beside");
        drop(map);
        drop(fresh);
    }
}
