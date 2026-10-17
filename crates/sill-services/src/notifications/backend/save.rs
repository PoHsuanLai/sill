//! When the history is written. Every change to it asks for a save, and a save is a durable
//! write (the file and its folder are synced), which takes as long as the disk takes. So the
//! asks are gathered: the first one starts a short wait, the ones that follow join it, and one
//! write on the blocking pool covers them all. The live loop never waits on the disk, and an app
//! that posts in a loop costs one write a second.

use std::path::PathBuf;
use std::time::Duration;

use sill_model::notifications::StoredHistory;
use tokio::task::JoinHandle;
use tokio::time::Instant;

use super::history_file;

/// How long after the first change the history is written: a burst of changes is one write, and a
/// quit still flushes ([`Saver::finish`]), so only a crash in this second loses a change.
pub(super) const SAVE_AFTER: Duration = Duration::from_secs(1);

/// The history's pending write.
#[derive(Debug)]
pub(super) struct Saver {
    file: PathBuf,
    after: Duration,
    /// When the pending write starts; none when nothing is pending.
    due: Option<Instant>,
    /// The write on the blocking pool, until it is found finished.
    writing: Option<JoinHandle<()>>,
}

impl Saver {
    pub(super) fn new(file: PathBuf, after: Duration) -> Saver {
        Saver {
            file,
            after,
            due: None,
            writing: None,
        }
    }

    /// The history changed. A write already waiting for its time covers this change too.
    pub(super) fn request(&mut self) {
        self.due.get_or_insert_with(|| Instant::now() + self.after);
    }

    /// Resolves when a write should start: one is pending, its time has come and the one before
    /// it has finished (two writes of one file never overlap). Never resolves when nothing is
    /// pending. Cancel-safe.
    pub(super) async fn ready(&mut self) {
        if let Some(writing) = self.writing.as_mut() {
            let _ = writing.await;
            self.writing = None;
        }
        match self.due {
            Some(at) => tokio::time::sleep_until(at).await,
            None => std::future::pending().await,
        }
    }

    /// Write `stored` on the blocking pool; what changes meanwhile asks again.
    pub(super) fn start(&mut self, stored: StoredHistory) {
        self.due = None;
        let file = self.file.clone();
        self.writing = Some(tokio::task::spawn_blocking(move || write(&file, &stored)));
    }

    /// Whether a change is waiting to be written, or a write has not finished.
    pub(super) fn busy(&self) -> bool {
        self.due.is_some() || self.writing.is_some()
    }

    /// The loop is ending: wait for the write in flight, then write `stored` at once if a change
    /// was still waiting.
    pub(super) async fn finish(&mut self, stored: impl FnOnce() -> StoredHistory) {
        if let Some(writing) = self.writing.take() {
            let _ = writing.await;
        }
        if self.due.take().is_some() {
            let (file, stored) = (self.file.clone(), stored());
            let _ = tokio::task::spawn_blocking(move || write(&file, &stored)).await;
        }
    }
}

fn write(file: &std::path::Path, stored: &StoredHistory) {
    if let Err(error) = history_file::save(file, stored) {
        eprintln!("sill: notifications: saving the history: {error}");
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use sill_model::notifications::{NotificationId, StoredHistory};
    use sill_testkit::TempDir;

    use super::{Saver, history_file};

    const SHORT: Duration = Duration::from_millis(30);
    const LONG: Duration = Duration::from_secs(60);

    fn stored(last: u32) -> StoredHistory {
        StoredHistory {
            last_id: NotificationId(last),
            ..StoredHistory::default()
        }
    }

    #[tokio::test]
    async fn nothing_is_due_until_a_change_asks_and_asks_are_gathered() {
        let dir = TempDir::new("notes-save");
        let mut saver = Saver::new(dir.path().join("h.json"), SHORT);
        assert!(!saver.busy());
        assert!(
            tokio::time::timeout(SHORT * 3, saver.ready())
                .await
                .is_err(),
            "nothing pending, so nothing to start"
        );
        for _ in 0..5 {
            saver.request();
        }
        assert!(saver.busy());
        saver.ready().await;
        saver.start(stored(5));
        // The write is not waited on by `start`; `ready` finds it finished and has nothing due.
        assert!(
            tokio::time::timeout(SHORT * 3, saver.ready())
                .await
                .is_err()
        );
        assert!(
            !saver.busy(),
            "the five asks were one write, and it is done"
        );
        assert_eq!(
            history_file::load(&dir.path().join("h.json")).last_id,
            NotificationId(5)
        );
    }

    #[tokio::test]
    async fn a_change_still_waiting_when_the_loop_ends_is_written_then() {
        let dir = TempDir::new("notes-save-finish");
        let file = dir.path().join("h.json");
        let mut saver = Saver::new(file.clone(), LONG);
        saver.request();
        saver.finish(|| stored(9)).await;
        assert_eq!(history_file::load(&file).last_id, NotificationId(9));
    }

    #[tokio::test]
    async fn the_loop_ending_with_nothing_pending_writes_nothing() {
        let dir = TempDir::new("notes-save-idle");
        let file = dir.path().join("h.json");
        Saver::new(file.clone(), SHORT).finish(|| stored(1)).await;
        assert!(!file.exists());
    }
}
