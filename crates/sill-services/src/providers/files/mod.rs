//! Documents and folders under the home directory (M9 lane a, FINDINGS "M9 freeze" F648;
//! Spotlight's "Documents"), from whichever index `launcher.files_backend` picks
//! ([`choice::pick`]): Tracker 3, Baloo, or `fd`/`plocate` within a time budget.
//!
//! The freeze's `query` answers nothing; lane a fills it: nothing typed asks no index; with
//! the policy's `FilesChoice::Off` or no index picked, nothing; otherwise one [`FileAsk`] with
//! the policy's budget under `$HOME`, its hits ranked by [`hits::file_items`] and pushed as
//! one batch (a `Partial` answer is pushed too). Enter opens the file (`Activation::Open`),
//! "Reveal in Files" (⌘R) its folder, "Copy Path" (⌘C) copies the path.
//!
//! Tests never read the person's index or home directory: they use [`FakeIndex`] and temp
//! directories; a live backend test only checks that the backend exists (`choice::detect`).

pub mod baloo;
pub mod choice;
pub mod fake;
pub mod hits;
pub mod index;
pub mod tracker;
pub mod walk;

pub use fake::FakeIndex;
pub use index::{FileAnswer, FileAsk, FileHit, FileIndex, HitLimit, IndexFault, IndexKind};

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use sill_launcher::{
    Activation, Choice, Frecency, Item, OpenTarget, Provider, ProviderKind, Query, ResultSink,
    Subject,
};
use tokio::sync::watch;

use crate::results::LauncherPolicy;
use hits::{COPY_PATH_ACTION, REVEAL_ACTION};

/// Where the provider's index comes from.
#[derive(Clone)]
pub enum IndexSource {
    /// Picked from the policy and this machine ([`choice::detect`], once).
    Detect,
    /// Always this one (a test's [`FakeIndex`]).
    Given(Arc<dyn FileIndex>),
}

impl std::fmt::Debug for IndexSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IndexSource::Detect => f.write_str("Detect"),
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
        let _ = (query, sink);
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
    use crate::results::LauncherPolicy;

    fn provider() -> FilesProvider {
        FilesProvider {
            policy: watch::channel(LauncherPolicy::default()).1,
            frecency: Arc::new(Mutex::default()),
            source: IndexSource::Detect,
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
