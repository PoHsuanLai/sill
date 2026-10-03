//! What a launcher preview names that is not a file: a clipboard image's bytes from the clipboard
//! service's history. A file is previewed by the viewer's own parts, off the UI thread, in
//! `sill-launcher-ui` (`anyview_peek`); nothing here reads one.

use sill_launcher::ClipKey;

use sill_model::clipboard::{ClipContent, ClipImage, Clipboard};

/// The image kept under `key`, if the history still holds it as an image.
pub fn clip_image(history: &Clipboard, key: ClipKey) -> Option<ClipImage> {
    history
        .entries
        .iter()
        .find(|entry| entry.key == key)
        .and_then(|entry| match &entry.content {
            ClipContent::Image(image) => Some(image.clone()),
            ClipContent::Text(_) => None,
        })
}
