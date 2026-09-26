//! Loading what a preview names off the UI thread (F644): a text file's head through
//! `sill_services::preview::read_excerpt` on a thread of its own, the answer landing in a
//! signal keyed by the path it answers, so a late answer for a row the selection has left is
//! never shown. A clipboard image comes from the clipboard service's history, already in memory.

use std::path::PathBuf;

use dioxus::prelude::*;
use sill_launcher::{ClipKey, Excerpt, ExcerptLimit};
use sill_services::preview::{PreviewFault, clip_image, read_excerpt};

use crate::hooks::use_clipboard;

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

/// The PNG bytes of the clipboard image kept under `key`, while the history holds it.
pub fn use_clip_png(key: Option<ClipKey>) -> Option<Vec<u8>> {
    let history = use_clipboard();
    let key = key?;
    let image = clip_image(&history.read(), key)?;
    Some(image.bytes.to_vec())
}
