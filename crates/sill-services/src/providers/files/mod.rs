//! Documents and folders under the home directory (Spotlight's "Documents"), from whichever
//! index `launcher.files_backend` picks ([`choice::pick`]): Tracker 3, or `fd`/`plocate` within
//! a time budget.
//!
//! Nothing typed asks no index; with
//! the policy's `FilesChoice::Off` or no index picked, nothing; otherwise one [`FileAsk`] with
//! the policy's budget under `$HOME`, its hits ranked by [`hits::file_items`] and pushed as
//! one batch (a `Partial` answer is pushed too). Enter opens the file (`Activation::Open`),
//! "Reveal in Files" (⌘R) its folder, "Copy Path" (⌘C) copies the path.
//!
//! Tests never read the person's index or home directory: they use [`FakeIndex`] and temp
//! directories; a live backend test only checks that the backend exists (`choice::detect`).

pub(crate) mod bus;
pub mod choice;
mod cursor;
pub mod fake;
pub mod hits;
pub mod index;
mod program;
#[cfg(test)]
mod query_tests;
mod stat;
pub mod tracker;
pub mod walk;

pub use fake::FakeIndex;
pub use index::{FileAnswer, FileAsk, FileHit, FileIndex, HitLimit, IndexFault, IndexKind};

use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use sill_launcher::{
    Activation, Batch, Choice, Frecency, Item, Needle, OpenTarget, Provider, ProviderKind, Query,
    RankWeights, ResultSink, Subject, UnixTime, matching,
};
use tokio::sync::watch;

use hits::{COPY_PATH_ACTION, REVEAL_ACTION};
use sill_model::results::{FilesChoice, LauncherPolicy};
use tracker::TrackerIndex;

use crate::env::Bus;
use walk::{WalkIndex, Walker};

/// Where the provider's index comes from.
#[derive(Clone)]
pub enum IndexSource {
    /// Picked from the policy and this machine ([`choice::detect`], once), Tracker on this
    /// session bus.
    Detect(Bus),
    /// Always this one (a test's [`FakeIndex`]).
    Given(Arc<dyn FileIndex>),
}

impl std::fmt::Debug for IndexSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IndexSource::Detect(bus) => write!(f, "Detect({bus:?})"),
            IndexSource::Given(index) => write!(f, "Given({:?})", index.kind()),
        }
    }
}

/// The files provider.
#[derive(Debug, Clone)]
pub struct FilesProvider {
    pub policy: watch::Receiver<LauncherPolicy>,
    /// The runner's history: a file chosen before ranks higher.
    pub frecency: Arc<Mutex<Frecency>>,
    pub source: IndexSource,
    /// Where hits must live (the home directory).
    pub home: PathBuf,
}

impl Provider for FilesProvider {
    fn kind(&self) -> ProviderKind {
        ProviderKind::Files
    }

    fn query(&self, query: &Query, sink: &mut dyn ResultSink) {
        let text = query.text.trim();
        if text.is_empty() || query.is_cancelled() {
            return;
        }
        let policy = self.policy.borrow().clone();
        let Some(index) = index_for(&self.source, policy.files) else {
            return;
        };
        let ask = FileAsk {
            text: text.to_owned(),
            under: self.home.clone(),
            limit: ASK_LIMIT,
            budget: policy.files_budget,
        };
        let hits = match index.search(&ask, query) {
            FileAnswer::Hits(hits) | FileAnswer::Partial(hits) => hits,
            FileAnswer::Unavailable(_) => return,
        };
        if query.is_cancelled() {
            return;
        }
        let items = hits::file_items(
            &hits,
            &Needle::parse(text),
            &self.home,
            &snapshot(&self.frecency),
            unix_now(),
            &RankWeights::DEFAULT,
            &mut matching::matcher(),
        );
        sink.push(Batch {
            provider: ProviderKind::Files,
            generation: query.generation,
            items,
        });
    }

    fn activate(&self, item: &Item, choice: &Choice) -> Activation {
        let Subject::File(file) = &item.subject else {
            return Activation::Nothing;
        };
        match choice {
            Choice::Primary => Activation::Open {
                target: OpenTarget::File(file.path.clone()),
            },
            Choice::Action(id) if id.0 == REVEAL_ACTION => Activation::Open {
                target: OpenTarget::Reveal(file.path.clone()),
            },
            Choice::Action(id) if id.0 == COPY_PATH_ACTION => Activation::Copy {
                text: file.path.to_string_lossy().into_owned(),
            },
            Choice::Action(_) => Activation::Nothing,
        }
    }
}

/// Most hits one ask brings back: the Documents section publishes at most 20 rows, and a hit
/// the name match drops leaves room for another.
const ASK_LIMIT: HitLimit = HitLimit(40);

/// What this machine has, asked once per process ([`choice::detect`] makes a bus call):
/// `FilesProvider`'s fields have no slot for a cache, and the daemon builds one provider, so
/// once per process is once per provider.
static DETECTED: OnceLock<choice::Detected> = OnceLock::new();

/// The index `source` and `files` name, or none (`Off`, or nothing on this machine fits).
fn index_for(source: &IndexSource, files: FilesChoice) -> Option<Arc<dyn FileIndex>> {
    if files == FilesChoice::Off {
        return None;
    }
    match source {
        IndexSource::Given(index) => Some(Arc::clone(index)),
        IndexSource::Detect(session) => {
            let detected = DETECTED.get_or_init(|| choice::detect(session));
            choice::pick(files, detected).and_then(|kind| index_of(kind, session))
        }
    }
}

fn index_of(kind: IndexKind, session: &Bus) -> Option<Arc<dyn FileIndex>> {
    match kind {
        IndexKind::Tracker => Some(Arc::new(TrackerIndex {
            session: session.clone(),
        })),
        IndexKind::Fd => Some(Arc::new(WalkIndex { walker: Walker::Fd })),
        IndexKind::Plocate => Some(Arc::new(WalkIndex {
            walker: Walker::Plocate,
        })),
        IndexKind::Fake => None,
    }
}

/// A read-only copy of the history; a poisoned lock still yields the data.
fn snapshot(frecency: &Mutex<Frecency>) -> Frecency {
    frecency
        .lock()
        .map(|guard| guard.clone())
        .unwrap_or_else(|poisoned| poisoned.into_inner().clone())
}

fn unix_now() -> UnixTime {
    UnixTime(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |since| since.as_secs()),
    )
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::sync::{Arc, Mutex};

    use sill_launcher::{
        ActionId, Activation, Choice, FileFacts, IconRef, Item, ItemId, Mime, OpenTarget, Provider,
        ProviderKind, Score, Subject, UnixTime,
    };
    use tokio::sync::watch;

    use super::{FilesProvider, IndexSource};
    use sill_model::results::LauncherPolicy;

    fn provider() -> FilesProvider {
        FilesProvider {
            policy: watch::channel(LauncherPolicy::default()).1,
            frecency: Arc::new(Mutex::default()),
            source: IndexSource::Detect(crate::env::Bus::session(crate::env::BusRoute::Absent)),
            home: PathBuf::from("/home/p"),
        }
    }

    #[test]
    fn a_file_row_opens_reveals_and_copies_its_path() {
        let row = Item {
            id: ItemId("/home/p/a.pdf".to_owned()),
            provider: ProviderKind::Files,
            title: "a.pdf".to_owned(),
            subtitle: "~".to_owned(),
            icon: IconRef::None,
            score: Score(0),
            actions: Vec::new(),
            subject: Subject::File(FileFacts {
                path: PathBuf::from("/home/p/a.pdf"),
                mime: Mime("application/pdf".to_owned()),
                modified: UnixTime(0),
            }),
        };
        let action = |id: &str| Choice::Action(ActionId(id.to_owned()));
        let path = PathBuf::from("/home/p/a.pdf");
        let cases = [
            (
                Choice::Primary,
                Activation::Open {
                    target: OpenTarget::File(path.clone()),
                },
            ),
            (
                action("reveal"),
                Activation::Open {
                    target: OpenTarget::Reveal(path),
                },
            ),
            (
                action("copy-path"),
                Activation::Copy {
                    text: "/home/p/a.pdf".to_owned(),
                },
            ),
            (action("other"), Activation::Nothing),
        ];
        for (choice, want) in cases {
            assert_eq!(provider().activate(&row, &choice), want, "{choice:?}");
        }
    }
}
