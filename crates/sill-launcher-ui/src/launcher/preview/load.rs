//! Loading what a preview names off the UI thread.
//!
//! A file is looked at by the viewer's own parts: [`anyview_peek::look`] says what it is (its
//! size, its date, its first bytes: the `Sniffed` that every peek takes) and makes
//! makes what the pane draws. That blocks, so it runs on the pane's one worker, which is the
//! viewer's own ([`anyview_peek::PeekWorker`]): a thread started with the pane and ended with it,
//! fed by a slot that holds the latest request only. The arrow keys ask for a new file on every
//! press; holding Down through fifty files peeks at the few the worker reaches, never fifty at
//! once (a PDF's first page is the slow one). The answer lands in a signal keyed by the path it
//! answers, so a late answer for a row the selection has left is never shown. The worker runs
//! each peek on a thread of its own with a stack large enough for the decoders, abandons one that
//! runs far past its time and refuses files while too many abandoned ones still run (see
//! `WorkerConfig`). A clipboard image comes from the clipboard service's history, already in
//! memory.

use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::Arc;

use anyview_core::{FilePath, Input, PeekBudget};
use anyview_fs::OnDisk;
use anyview_peek::{
    AnyPeeked, Body, NoStills, PeekFailure, PeekWork, PeekWorker, StillSource, Unavailable,
    WorkerConfig, look,
};
use dioxus::prelude::*;
use sill_launcher::ClipKey;
use sill_search::preview::clip_image;
use sill_services::ClipboardService;
use sill_ui_kit::hooks::use_service;
use sill_ui_kit::services::Services;
use tokio::sync::oneshot;

use super::frames::ThumbnailFrames;

/// What a peek of a file may spend: it reads a PDF whole, and a picture is fitted to a pane's
/// worth of pixels (`PANE` is 328 by 220 px, drawn at up to twice that). Constants until quire's
/// settings have `viewer.peek.*` keys (the viewer's FINDINGS).
pub const PANE_BUDGET: PeekBudget = PeekBudget::DEFAULT;

/// Why a file could not be previewed. Carries no path or content.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PreviewFault {
    Missing,
    Unreadable,
}

/// A file's peek as the pane has it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Peeking {
    /// Still being read (past a frame, the pane's `Pending` moment).
    Reading,
    /// What the pane draws.
    Ready(Arc<AnyPeeked>),
    Failed(PreviewFault),
}

/// How a file is looked at, blocking: the viewer's parts in the launcher, a test's own where it
/// needs to hold a look open.
pub trait Look: Send + Sync + 'static {
    /// The peek of the file at `path`.
    fn at(&self, path: &Path) -> Result<Arc<AnyPeeked>, PreviewFault>;
}

/// The viewer's parts: look at the file (probe, then peek as sniffed). A video's frame, which the
/// peek does not decode, comes from `frames`.
#[derive(Debug, Clone)]
pub struct ViewerLook {
    frames: Arc<dyn StillSource>,
}

impl ViewerLook {
    /// A look that takes a video's picture from `frames`.
    pub fn new(frames: Arc<dyn StillSource>) -> ViewerLook {
        ViewerLook { frames }
    }

    /// A look with no source of video frames: a video without a cover shows its facts.
    pub fn without_frames() -> ViewerLook {
        ViewerLook::new(Arc::new(NoStills))
    }
}

impl Look for ViewerLook {
    fn at(&self, path: &Path) -> Result<Arc<AnyPeeked>, PreviewFault> {
        let file = FilePath::new(path).map_err(|_| PreviewFault::Unreadable)?;
        let peeking = anyview_peek::Peeking::pane()
            .with_budget(PANE_BUDGET)
            .with_stills(Arc::clone(&self.frames));
        let card = look(file.on_disk(), &peeking);
        // A path that is not there is a fault of its own; every other failure is a card that says
        // why, beside the file's facts.
        match card.body {
            Body::Unavailable(Unavailable::Missing) => Err(PreviewFault::Missing),
            _ => Ok(Arc::new(card)),
        }
    }
}

/// How the pane looks at files, as a root context; the viewer's own over the thumbnail cache of
/// the daemon's `Dirs` when none is provided.
#[derive(Clone)]
pub struct Looking(pub Arc<dyn Look>);

impl std::fmt::Debug for Looking {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Looking")
    }
}

/// The viewer's look over the thumbnail cache in the services' cache directory; without services
/// (a root that mounts the pane alone) it has no video frames.
fn cached_look() -> Arc<dyn Look> {
    let look = try_consume_context::<Services>().map_or_else(ViewerLook::without_frames, |s| {
        ViewerLook::new(Arc::new(ThumbnailFrames::in_cache(&s.dirs.cache)))
    });
    Arc::new(look)
}

/// What a look comes to for the pane.
type Looked = Result<Arc<AnyPeeked>, PreviewFault>;

/// The pane's worker: the viewer's, looking at files as `look` says. Without a thread (the system
/// refused) every request waits unanswered and the pane stays in its reading state rather than
/// the launcher failing.
fn start_worker(look: Arc<dyn Look>) -> PeekWorker<Looked> {
    let work: PeekWork<Looked> = Arc::new(move |input: Input| match input.path() {
        Some(file) => look.at(file.as_path()),
        None => Err(PreviewFault::Unreadable),
    });
    let failed: PeekFailure<Looked> = Arc::new(|_| Err(PreviewFault::Unreadable));
    PeekWorker::spawn(
        WorkerConfig::default().with_name("launcher-peek"),
        work,
        failed,
    )
}

/// Ask `worker` for the look at `path`, replacing the request that has not started: its asker
/// hears nothing (its receiver is closed), and its answer would have been stale.
fn ask(worker: &PeekWorker<Looked>, path: &Path) -> oneshot::Receiver<Looked> {
    let (reply, answer) = oneshot::channel();
    match FilePath::new(path) {
        Ok(file) => worker.ask(file.on_disk(), move |looked| {
            let _ = reply.send(looked);
        }),
        Err(_) => {
            let _ = reply.send(Err(PreviewFault::Unreadable));
        }
    }
    answer
}

/// The peek of the file at `path` (`None`: the pane shows no file).
pub fn use_peek(path: Option<PathBuf>) -> Option<Peeking> {
    let mut answer = use_signal(|| None::<(PathBuf, Result<Arc<AnyPeeked>, PreviewFault>)>);
    let worker = use_hook(|| {
        let look = try_consume_context::<Looking>().map_or_else(cached_look, |looking| looking.0);
        Rc::new(start_worker(look))
    });
    let mut asked = use_hook(|| CopyValue::new(None::<PathBuf>));
    let path = path?;
    if asked.peek().as_ref() != Some(&path) {
        asked.set(Some(path.clone()));
        let reply = ask(&worker, &path);
        let answering = path.clone();
        spawn(async move {
            if let Ok(result) = reply.await {
                answer.set(Some((answering, result)));
            }
        });
    }
    let peeking = match answer.read().as_ref() {
        Some((answered, Ok(peeked))) if *answered == path => Peeking::Ready(Arc::clone(peeked)),
        Some((answered, Err(fault))) if *answered == path => Peeking::Failed(*fault),
        _ => Peeking::Reading,
    };
    Some(peeking)
}

/// The PNG bytes of the clipboard image kept under `key`, while the history holds it.
pub fn use_clip_png(key: Option<ClipKey>) -> Option<Vec<u8>> {
    let history = use_service::<ClipboardService>();
    let key = key?;
    let image = clip_image(&history.read(), key)?;
    // quire takes a picture as PNG bytes; the service keeps PNG when it is offered.
    (image.mime.0 == "image/png").then(|| image.bytes.to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_missing_file_is_a_fault_and_a_file_is_a_peek() {
        let dir = scratch_dir();
        let present = dir.join("notes.txt");
        std::fs::write(&present, "first line\nsecond\n").unwrap_or_else(|e| panic!("{e}"));
        let peeked = ViewerLook::without_frames()
            .at(&present)
            .unwrap_or_else(|e| panic!("{e:?}"));
        assert_eq!(peeked.name, "notes.txt");
        assert_eq!(
            ViewerLook::without_frames().at(&dir.join("absent.txt")),
            Err(PreviewFault::Missing)
        );
        std::fs::remove_dir_all(&dir).unwrap_or_else(|e| panic!("{e}"));
    }

    /// A look that panics or answers, by the file's name.
    struct Scripted;

    impl Look for Scripted {
        fn at(&self, path: &Path) -> Result<Arc<AnyPeeked>, PreviewFault> {
            match path.to_str() {
                Some("/panics") => panic!("a decoder fell over"),
                _ => Err(PreviewFault::Missing),
            }
        }
    }

    fn answered(worker: &PeekWorker<Looked>, path: &str) -> Looked {
        ask(worker, Path::new(path))
            .blocking_recv()
            .unwrap_or_else(|e| panic!("no answer: {e}"))
    }

    #[test]
    fn a_panicking_peek_is_a_fault_and_the_worker_answers_the_next_file() {
        let worker = start_worker(Arc::new(Scripted));
        assert_eq!(answered(&worker, "/panics"), Err(PreviewFault::Unreadable));
        assert_eq!(answered(&worker, "/next"), Err(PreviewFault::Missing));
    }

    #[test]
    fn a_path_that_is_not_absolute_is_a_fault_without_a_look() {
        let worker = start_worker(Arc::new(Scripted));
        assert_eq!(answered(&worker, "relative"), Err(PreviewFault::Unreadable));
    }

    fn scratch_dir() -> PathBuf {
        let dir = std::env::temp_dir().join(format!("sill-peek-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap_or_else(|e| panic!("{e}"));
        dir
    }
}
