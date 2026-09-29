//! Loading what a launcher preview names: a text file's first lines and a clipboard image's bytes.
//! `sill_launcher::preview_for` decides what to show; this reads it, off the UI thread, bounded (a
//! preview never reads more than [`HEAD_BYTES`] of a file). A PDF page and an image file are drawn
//! by quire from their paths and need nothing here.

use std::io::Read;
use std::path::Path;

use sill_launcher::{ClipKey, Excerpt, ExcerptLimit, excerpt_of};

use crate::clipboard::{ClipContent, ClipImage, Clipboard};

/// The most of a text file a preview reads.
pub const HEAD_BYTES: usize = 16 * 1024;

/// Why a preview could not be loaded. Carries no path or content.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PreviewFault {
    Missing,
    Unreadable,
}

/// The first lines of the text file at `path` within `limit`: at most [`HEAD_BYTES`] read, then
/// `sill_launcher::excerpt_of`. Blocking.
pub fn read_excerpt(path: &Path, limit: ExcerptLimit) -> Result<Excerpt, PreviewFault> {
    let file = std::fs::File::open(path).map_err(fault_of)?;
    let mut head = Vec::with_capacity(HEAD_BYTES);
    file.take(HEAD_BYTES as u64)
        .read_to_end(&mut head)
        .map_err(fault_of)?;
    Ok(excerpt_of(&head, limit))
}

fn fault_of(error: std::io::Error) -> PreviewFault {
    match error.kind() {
        std::io::ErrorKind::NotFound => PreviewFault::Missing,
        _ => PreviewFault::Unreadable,
    }
}

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

#[cfg(test)]
mod tests {
    use std::io::Write;

    use sill_launcher::{Excerpt, ExcerptLimit};

    use super::{PreviewFault, read_excerpt};

    #[test]
    fn a_file_head_becomes_an_excerpt_and_a_missing_file_a_fault() {
        let dir = std::env::temp_dir().join(format!("sill-preview-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("temp dir");
        let path = dir.join("notes.md");
        let mut file = std::fs::File::create(&path).expect("create");
        writeln!(file, "# Notes\nfirst line").expect("write");
        assert_eq!(
            read_excerpt(&path, ExcerptLimit::PANE),
            Ok(Excerpt("# Notes\nfirst line".to_owned()))
        );
        assert_eq!(
            read_excerpt(&dir.join("absent.md"), ExcerptLimit::PANE),
            Err(PreviewFault::Missing)
        );
        std::fs::remove_dir_all(&dir).expect("clean up");
    }
}
