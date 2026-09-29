//! The file-index seam: Tracker 3 and a walk under the home directory are two implementations
//! behind [`FileIndex`], and a test's [`FakeIndex`](super::fake::FakeIndex) is a third. Each
//! answers one [`FileAsk`] within its
//! budget; the provider ranks the hits itself (`hits.rs`), so every backend ranks alike.

use std::path::PathBuf;
use std::time::Duration;

use sill_launcher::{Mime, Query, UnixTime};

/// Which index answered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IndexKind {
    Tracker,
    Fd,
    Plocate,
    Fake,
}

/// Most hits one ask returns.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct HitLimit(pub u16);

/// One search.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileAsk {
    /// The typed text, trimmed; never empty (an empty query asks no index).
    pub text: String,
    /// Only hits under this folder (the home directory).
    pub under: PathBuf,
    pub limit: HitLimit,
    /// Past this the ask gives up and answers what it has.
    pub budget: Duration,
}

/// One file an index found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileHit {
    pub path: PathBuf,
    /// The file name, as shown.
    pub name: String,
    /// The index's type, else guessed from the name (`mime_guess`), else
    /// `application/octet-stream`; `inode/directory` for a folder.
    pub mime: Mime,
    pub modified: UnixTime,
}

/// What an ask produced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileAnswer {
    /// Every hit, within the budget.
    Hits(Vec<FileHit>),
    /// The budget ran out; these are the hits found by then.
    Partial(Vec<FileHit>),
    /// The index is not there or failed (no bus, the miner is not running, the program is
    /// missing or exited badly).
    Unavailable(IndexFault),
}

/// Why an index could not answer. Carries no path or query (it may be logged).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IndexFault {
    NoBus,
    NotRunning,
    NoProgram,
    Failed,
    Cancelled,
}

/// A file index.
pub trait FileIndex: Send + Sync {
    /// Which index this is.
    fn kind(&self) -> IndexKind;

    /// Answer `ask`, stopping early once `query` is cancelled (a newer keystroke). Called on
    /// the blocking pool (the runner's `spawn_blocking`), so it may block up to `ask.budget`.
    fn search(&self, ask: &FileAsk, query: &Query) -> FileAnswer;
}
