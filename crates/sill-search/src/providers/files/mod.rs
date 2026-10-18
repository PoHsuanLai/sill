//! Documents and folders under the home directory (Spotlight's "Documents"), from whichever
//! index `launcher.files_backend` picks ([`choice::pick`]): Tracker 3, or `fd`/`plocate` within
//! a time budget.
//!
//! Nothing typed asks no index; with the policy's `FilesChoice::Off`, nothing; otherwise one
//! [`FileAsk`] with the policy's budget under `$HOME` (none when no index is picked), plus the
//! files the viewer remembers (`viewed.rs`: they rank with the launcher's frecency, and the ones
//! the index did not bring are rows of their own), ranked by [`hits::file_items`] and pushed as
//! one batch (a `Partial` answer is pushed too). A row carries the shared list of actions for
//! its kind (`actions.rs`); Enter is its Open: a folder in the file manager, a file in the
//! viewer.
//!
//! Tests never read the person's index or home directory: they use [`FakeIndex`] and temp
//! directories; a live backend test only checks that the backend exists (`choice::detect`).

pub mod actions;
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
mod viewed;
pub mod walk;

pub use fake::FakeIndex;
pub use index::{FileAnswer, FileAsk, FileHit, FileIndex, HitLimit, IndexFault, IndexKind};

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

use anyview_core::FileAction;
use anyview_store::read_history;
use ds_core::word::Word;
use sill_launcher::{
    ActionId, Activation, Batch, Choice, Frecency, Item, Needle, Provider, ProviderKind, Query,
    RankWeights, ResultSink, Subject, matching,
};
use tokio::sync::watch;

use crate::clock::{modified_of, unix_now};
use crate::providers::locked::copy_of;

use sill_model::results::{FilesChoice, LauncherPolicy};
use tracker::TrackerIndex;

use sill_services::env::Bus;
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
    /// The viewer's store, which holds the history of files it opened (`<data>/anyview`).
    pub viewer_store: PathBuf,
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
        if policy.files == FilesChoice::Off {
            return;
        }
        let indexed = self.index_hits(&policy, text, query);
        if query.is_cancelled() {
            return;
        }
        let answered = indexed.is_some();
        let mut hits = indexed.unwrap_or_default();
        let history = read_history(&self.viewer_store);
        let views = history.entries();
        let known: BTreeSet<PathBuf> = hits.iter().map(|hit| hit.path.clone()).collect();
        let (needle, mut matcher) = (Needle::parse(text), matching::matcher());
        let viewed_hits: Vec<FileHit> = viewed::viewed_hits(
            views,
            |name| hits::name_matches(&needle, name, &mut matcher),
            modified_of,
        )
        .into_iter()
        .filter(|hit| !known.contains(&hit.path))
        .collect();
        if !answered && viewed_hits.is_empty() {
            return;
        }
        hits.extend(viewed_hits);
        let frecency = viewed::fold_views(copy_of(&self.frecency), views);
        let items = hits::file_items(
            &hits,
            &needle,
            &self.home,
            &frecency,
            unix_now(),
            &RankWeights::DEFAULT,
            &mut matcher,
        );
        let items = viewed::with_places(items, views);
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
            Choice::Primary => actions::activation_of(&open_action(), file),
            Choice::Action(id) => actions::activation_of(id, file),
        }
    }
}

impl FilesProvider {
    /// What the index brings for `text` within the policy's budget: `None` when no index is
    /// picked or it cannot answer.
    fn index_hits(
        &self,
        policy: &LauncherPolicy,
        text: &str,
        query: &Query,
    ) -> Option<Vec<FileHit>> {
        let index = index_for(&self.source, policy.files)?;
        let ask = FileAsk {
            text: text.to_owned(),
            under: self.home.clone(),
            limit: ASK_LIMIT,
            budget: policy.files_budget,
        };
        match index.search(&ask, query) {
            FileAnswer::Hits(hits) | FileAnswer::Partial(hits) => Some(hits),
            FileAnswer::Unavailable(_) => None,
        }
    }
}

/// Enter's action: the same as the row's Open.
fn open_action() -> ActionId {
    ActionId(FileAction::Open.slug().to_owned())
}

/// Most hits one ask brings back: the Documents section publishes at most 20 rows, and a hit
/// the name match drops leaves room for another.
const ASK_LIMIT: HitLimit = HitLimit(40);

/// What this machine has, and when it was asked ([`choice::detect`] makes a bus call):
/// `FilesProvider`'s fields have no slot for a cache, and the daemon builds one provider, so
/// one per process is one per provider.
static DETECTED: Mutex<Option<(Instant, choice::Detected)>> = Mutex::new(None);

/// How long a detection is believed. An indexer that starts after the shell, or a bus that was
/// not up yet at the first keystroke, is found within this; between asks nothing touches the bus.
const DETECTION_TTL: Duration = Duration::from_secs(60);

/// What this machine has, asked again once [`DETECTION_TTL`] has passed.
fn detected(session: &Bus) -> choice::Detected {
    let mut cache = DETECTED.lock().unwrap_or_else(PoisonError::into_inner);
    match *cache {
        Some((at, known)) if at.elapsed() < DETECTION_TTL => known,
        Some(_) | None => {
            let known = choice::detect(session);
            *cache = Some((Instant::now(), known));
            known
        }
    }
}

/// The index `source` and `files` name, or none (`Off`, or nothing on this machine fits).
fn index_for(source: &IndexSource, files: FilesChoice) -> Option<Arc<dyn FileIndex>> {
    if files == FilesChoice::Off {
        return None;
    }
    match source {
        IndexSource::Given(index) => Some(Arc::clone(index)),
        IndexSource::Detect(session) => {
            choice::pick(files, &detected(session)).and_then(|kind| index_of(kind, session))
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

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::sync::{Arc, Mutex};

    use sill_launcher::{
        ActionId, Activation, Choice, FileFacts, Generation, IconRef, Item, ItemId, Mime,
        OpenTarget, Provider, ProviderKind, Score, Subject, UnixTime, ViewerOpen,
    };
    use tokio::sync::watch;

    use super::{FilesProvider, IndexSource};
    use sill_model::results::LauncherPolicy;

    fn provider() -> FilesProvider {
        FilesProvider {
            policy: watch::channel(LauncherPolicy::default()).1,
            frecency: Arc::new(Mutex::default()),
            source: IndexSource::Detect(sill_services::env::Bus::session(
                sill_services::env::BusRoute::Absent,
            )),
            home: PathBuf::from("/home/p"),
            viewer_store: PathBuf::from("/home/p/.local/share/anyview"),
        }
    }

    fn row(path: &str, mime: &str) -> Item {
        Item {
            id: ItemId(path.to_owned()),
            provider: ProviderKind::Files,
            title: "a".to_owned(),
            subtitle: "~".to_owned(),
            icon: IconRef::None,
            score: Score(0),
            actions: Vec::new(),
            subject: Subject::File(FileFacts {
                path: PathBuf::from(path),
                mime: Mime(mime.to_owned()),
                modified: UnixTime(0),
            }),
        }
    }

    #[test]
    fn a_file_row_opens_in_the_viewer_reveals_and_copies_its_path() {
        let pdf = row("/home/p/a.pdf", "application/pdf");
        let folder = row("/home/p/docs", Mime::FOLDER);
        let action = |id: &str| Choice::Action(ActionId(id.to_owned()));
        let path = PathBuf::from("/home/p/a.pdf");
        let viewer = Activation::Open {
            target: OpenTarget::Viewer(ViewerOpen {
                file: path.clone(),
                search: Generation::default(),
                results: Vec::new(),
            }),
        };
        let cases = [
            ("enter on a file", &pdf, Choice::Primary, viewer.clone()),
            ("open on a file", &pdf, action("open"), viewer),
            (
                "enter on a folder",
                &folder,
                Choice::Primary,
                Activation::Open {
                    target: OpenTarget::File(PathBuf::from("/home/p/docs")),
                },
            ),
            (
                "reveal",
                &pdf,
                action("reveal_in_folder"),
                Activation::Open {
                    target: OpenTarget::Reveal(path),
                },
            ),
            (
                "copy the path",
                &pdf,
                action("copy_path"),
                Activation::Copy {
                    text: "/home/p/a.pdf".to_owned(),
                },
            ),
            (
                "an action the launcher does not carry",
                &pdf,
                action("share"),
                Activation::Nothing,
            ),
            (
                "an id that is no action",
                &pdf,
                action("other"),
                Activation::Nothing,
            ),
        ];
        for (name, row, choice, want) in cases {
            assert_eq!(provider().activate(row, &choice), want, "{name}");
        }
    }
}
