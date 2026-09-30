//! Loading what a preview names off the UI thread: a text file's head through
//! `sill_search::preview::read_excerpt` on a thread of its own, the answer landing in a
//! signal keyed by the path it answers, so a late answer for a row the selection has left is
//! never shown. A clipboard image comes from the clipboard service's history, already in memory.

use std::path::PathBuf;

use dioxus::prelude::*;
use ds::components::content::image_source::ImageSize;
use sill_launcher::{ClipKey, Excerpt, ExcerptLimit};
use sill_search::preview::{PreviewFault, clip_image, read_excerpt};

use sill_services::ClipboardService;
use sill_ui_kit::hooks::use_service;

/// A text file's head as the pane has it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Loaded {
    /// Still being read (past a frame, the pane's `Pending` moment).
    Reading,
    Text(Excerpt),
    Failed(PreviewFault),
}

/// The head of the text file at `path` (`None`: the pane shows no text file).
pub fn use_excerpt(path: Option<PathBuf>) -> Option<Loaded> {
    let mut answer = use_signal(|| None::<(PathBuf, Result<Excerpt, PreviewFault>)>);
    let mut asked = use_hook(|| CopyValue::new(None::<PathBuf>));
    let path = path?;
    if asked.peek().as_ref() != Some(&path) {
        asked.set(Some(path.clone()));
        let (tx, rx) = tokio::sync::oneshot::channel();
        let reading = path.clone();
        std::thread::spawn(move || {
            let _ = tx.send(read_excerpt(&reading, ExcerptLimit::PANE));
        });
        let answering = path.clone();
        spawn(async move {
            if let Ok(result) = rx.await {
                answer.set(Some((answering, result)));
            }
        });
    }
    let loaded = match answer.read().as_ref() {
        Some((answered, Ok(excerpt))) if *answered == path => Loaded::Text(excerpt.clone()),
        Some((answered, Err(fault))) if *answered == path => Loaded::Failed(*fault),
        _ => Loaded::Reading,
    };
    Some(loaded)
}

/// The pixel size of the image file at `path` (its header only, read on a thread of its own);
/// `None` until it is read, or when there is no image or it cannot be read.
pub fn use_image_size(path: Option<PathBuf>) -> Option<ImageSize> {
    let mut answer = use_signal(|| None::<(PathBuf, ImageSize)>);
    let mut asked = use_hook(|| CopyValue::new(None::<PathBuf>));
    let path = path?;
    if asked.peek().as_ref() != Some(&path) {
        asked.set(Some(path.clone()));
        let (tx, rx) = tokio::sync::oneshot::channel();
        let reading = path.clone();
        std::thread::spawn(move || {
            let _ = tx.send(image::image_dimensions(&reading).ok());
        });
        spawn(async move {
            if let Ok(Some((width, height))) = rx.await {
                answer.set(Some((path, ImageSize { width, height })));
            }
        });
        return None;
    }
    answer
        .read()
        .as_ref()
        .filter(|(answered, _)| *answered == path)
        .map(|(_, size)| *size)
}

/// The PNG bytes of the clipboard image kept under `key`, while the history holds it.
pub fn use_clip_png(key: Option<ClipKey>) -> Option<Vec<u8>> {
    let history = use_service::<ClipboardService>();
    let key = key?;
    let image = clip_image(&history.read(), key)?;
    // quire takes a picture as PNG bytes; the service keeps PNG when it is offered.
    (image.mime.0 == "image/png").then(|| image.bytes.to_vec())
}
