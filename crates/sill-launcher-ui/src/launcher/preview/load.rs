//! Loading what a preview names off the UI thread.
//!
//! A file is looked at by the viewer's own parts: [`anyview_peek::probe`] says what it is (its
//! size, its date, its first bytes: the `Sniffed` that every peek takes) and [`anyview_peek::peek_with`]
//! makes what the pane draws. That blocks, so it runs on the pane's one worker: a thread started
//! with the pane and ended with it, fed by a slot that holds the latest request only. The arrow
//! keys ask for a new file on every press; holding Down through fifty files peeks at the few the
//! worker reaches, never fifty at once (a PDF's first page is the slow one). The answer lands in
//! a signal keyed by the path it answers, so a late answer for a row the selection has left is
//! never shown. A clipboard image comes from the clipboard service's history, already in memory.

use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::{Arc, Condvar, Mutex, PoisonError};
use std::time::Duration;

use anyview_core::{ByteLen, FilePath, PeekBudget, PixelArea};
use anyview_peek::{AnyPeeked, NoFrames, PeekError, VideoFrames, peek_with, probe};
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
pub const PANE_BUDGET: PeekBudget = PeekBudget {
    bytes: ByteLen(64 * 1024 * 1024),
    pixels: PixelArea(1_000_000),
    time: Duration::from_millis(500),
};

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

/// The viewer's parts: probe the file, then peek at it as sniffed. A video's frame, which the
/// peek does not decode, comes from `frames`.
#[derive(Debug, Clone)]
pub struct ViewerLook {
    frames: Arc<dyn VideoFrames>,
}

impl ViewerLook {
    /// A look that takes a video's picture from `frames`.
    pub fn new(frames: Arc<dyn VideoFrames>) -> ViewerLook {
        ViewerLook { frames }
    }

    /// A look with no source of video frames: a video without a cover shows its facts.
    pub fn without_frames() -> ViewerLook {
        ViewerLook::new(Arc::new(NoFrames))
    }
}

impl Look for ViewerLook {
    fn at(&self, path: &Path) -> Result<Arc<AnyPeeked>, PreviewFault> {
        let file = FilePath::new(path).map_err(|_| PreviewFault::Unreadable)?;
        let probed = probe(&file).map_err(fault_of)?;
        Ok(Arc::new(peek_with(
            &probed.source,
            &probed.sniffed,
            &PANE_BUDGET,
            self.frames.as_ref(),
        )))
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

/// Probing fails only for a path that is not there or cannot be read; the other errors are the
/// peek's, which `peek` turns into a pane that says so.
fn fault_of(error: PeekError) -> PreviewFault {
    match error {
        PeekError::Missing { .. } => PreviewFault::Missing,
        PeekError::Unreadable { .. }
        | PeekError::Image(_)
        | PeekError::Text(_)
        | PeekError::Media { .. }
        | PeekError::Archive(_)
        | PeekError::Font(_)
        | PeekError::Pdf(_)
        | PeekError::Folder { .. }
        | PeekError::OverBudget { .. }
        | PeekError::WrongKind { .. } => PreviewFault::Unreadable,
    }
}

/// The request waiting for the worker, and whether the pane is gone.
#[derive(Default)]
struct Slot {
    waiting: Mutex<Waiting>,
    ready: Condvar,
}

#[derive(Default)]
struct Waiting {
    request: Option<Request>,
    ended: bool,
}

struct Request {
    path: PathBuf,
    reply: oneshot::Sender<Result<Arc<AnyPeeked>, PreviewFault>>,
}

/// The pane's worker: dropping it (the pane unmounts) ends its thread.
struct Worker(Arc<Slot>);

impl Worker {
    /// Start the thread, looking at files as `look` says. Without a thread (the system refused)
    /// every request waits unanswered and the pane stays in its reading state rather than the
    /// launcher failing.
    fn start(look: Arc<dyn Look>) -> Worker {
        let slot = Arc::new(Slot::default());
        let mine = Arc::clone(&slot);
        let _ = std::thread::Builder::new()
            .name("launcher-peek".to_owned())
            .spawn(move || work(&mine, look.as_ref()));
        Worker(slot)
    }

    /// Ask for the peek of `path`, replacing the request that has not started: its asker hears
    /// nothing, and its answer would have been stale.
    fn ask(&self, path: PathBuf) -> oneshot::Receiver<Result<Arc<AnyPeeked>, PreviewFault>> {
        let (reply, answer) = oneshot::channel();
        self.0
            .waiting
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .request = Some(Request { path, reply });
        self.0.ready.notify_one();
        answer
    }
}

impl Drop for Worker {
    fn drop(&mut self) {
        self.0
            .waiting
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .ended = true;
        self.0.ready.notify_one();
    }
}

/// Take the waiting request, peek, answer; until the pane is gone.
fn work(slot: &Slot, look: &dyn Look) {
    loop {
        let request = {
            let mut waiting = slot.waiting.lock().unwrap_or_else(PoisonError::into_inner);
            loop {
                if waiting.ended {
                    return;
                }
                match waiting.request.take() {
                    Some(request) => break request,
                    None => {
                        waiting = slot
                            .ready
                            .wait(waiting)
                            .unwrap_or_else(PoisonError::into_inner);
                    }
                }
            }
        };
        // An asker that moved on dropped its receiver; the answer then goes nowhere.
        let _ = request.reply.send(look.at(&request.path));
    }
}

/// The peek of the file at `path` (`None`: the pane shows no file).
pub fn use_peek(path: Option<PathBuf>) -> Option<Peeking> {
    let mut answer = use_signal(|| None::<(PathBuf, Result<Arc<AnyPeeked>, PreviewFault>)>);
    let worker = use_hook(|| {
        let look = try_consume_context::<Looking>().map_or_else(cached_look, |looking| looking.0);
        Rc::new(Worker::start(look))
    });
    let mut asked = use_hook(|| CopyValue::new(None::<PathBuf>));
    let path = path?;
    if asked.peek().as_ref() != Some(&path) {
        asked.set(Some(path.clone()));
        let reply = worker.ask(path.clone());
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
    fn a_waiting_request_is_replaced_by_the_next_and_its_asker_hears_nothing() {
        // No thread: the slot alone, as the worker would find it.
        let worker = Worker(Arc::new(Slot::default()));
        let mut first = worker.ask(PathBuf::from("/a"));
        let mut second = worker.ask(PathBuf::from("/b"));
        let taken = worker
            .0
            .waiting
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .request
            .take();
        assert_eq!(
            taken.as_ref().map(|request| request.path.clone()),
            Some(PathBuf::from("/b")),
            "the latest request waits"
        );
        assert!(
            matches!(first.try_recv(), Err(oneshot::error::TryRecvError::Closed)),
            "the replaced asker is told nothing is coming"
        );
        assert!(
            matches!(second.try_recv(), Err(oneshot::error::TryRecvError::Empty)),
            "the waiting one still waits for its answer"
        );
        drop(taken);
    }

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

    fn scratch_dir() -> PathBuf {
        let dir = std::env::temp_dir().join(format!("sill-peek-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap_or_else(|e| panic!("{e}"));
        dir
    }
}
