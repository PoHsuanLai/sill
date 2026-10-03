//! A [`Preview`] as quire's `PaneContent`, with what it names on disk loaded off the UI thread: a
//! file's peek through `load.rs` (the viewer's `anyview_peek`, drawn by its `Pane` in quire's
//! slot so one `PreviewPane` keeps its actions, its cue, its pending look and its entrance
//! whatever it shows), a clipboard image's bytes from the clipboard service.

use std::path::Path;
use std::sync::Arc;

use anyview_peek::{AnyPeeked, Pane};
use dioxus::prelude::*;
use ds::components::content::image_source::{ImageSize, ImageSource};
use ds::components::lists::preview::content::{PaneContent, PaneMono};
use ds::prelude::*;
use sill_launcher::{ClipKind, Mime, Preview};

use crate::launcher::preview::load::{Peeking, PreviewFault, use_clip_png, use_peek};

/// How far what a preview names has loaded.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) enum Load {
    /// All of it, or it needs no load.
    Ready,
    /// A file is still being looked at.
    Reading,
    /// It could not be read.
    Failed,
}

/// The content for `preview` (`icon`: the row's, for an app), and how far it has loaded. Hooks
/// run on every render, whatever the kind.
pub(super) fn use_content(preview: &Preview, icon: Option<IconSource>) -> (PaneContent, Load) {
    let file = match preview {
        Preview::File(file) => Some(file.path.clone()),
        _ => None,
    };
    let clip = match preview {
        Preview::Clipboard(facts) => match facts.kind {
            ClipKind::Image { .. } => Some(facts.key),
            ClipKind::Text { .. } => None,
        },
        _ => None,
    };
    let peeking = use_peek(file);
    let clip_png = use_clip_png(clip);
    let load = load_of(peeking.as_ref());
    let content = content_of(preview, icon, Loads { peeking, clip_png });
    (content, load)
}

/// How far a file's peek has loaded (`None`: the preview has no file).
fn load_of(peeking: Option<&Peeking>) -> Load {
    match peeking {
        Some(Peeking::Reading) => Load::Reading,
        Some(Peeking::Failed(_)) => Load::Failed,
        Some(Peeking::Ready(_)) | None => Load::Ready,
    }
}

/// What the hooks brought for the preview.
struct Loads {
    peeking: Option<Peeking>,
    clip_png: Option<Vec<u8>>,
}

fn content_of(preview: &Preview, icon: Option<IconSource>, loads: Loads) -> PaneContent {
    match preview {
        Preview::None => facts(Icon::Search, "No preview", Vec::new()),
        Preview::File(file) => match loads.peeking {
            Some(Peeking::Ready(peeked)) => peeked_slot(&peeked),
            Some(Peeking::Reading) | None => PaneContent::Slot {
                label: file_name(&file.path),
                element: rsx! {},
            },
            Some(Peeking::Failed(fault)) => unreadable(file, fault),
        },
        Preview::Text { excerpt } => PaneContent::Text {
            excerpt: excerpt.0.clone(),
            mono: PaneMono::Proportional,
        },
        Preview::App { name, detail, .. } => PaneContent::App {
            icon: icon.unwrap_or(IconSource::Glyph(Icon::Grid)),
            name: name.clone(),
            detail: (!detail.is_empty()).then(|| detail.clone()),
        },
        Preview::Clipboard(clip) => match (&clip.kind, loads.clip_png) {
            (ClipKind::Text { excerpt, .. }, _) => PaneContent::Text {
                excerpt: excerpt.0.clone(),
                mono: PaneMono::Monospace,
            },
            (ClipKind::Image { width, height, .. }, Some(bytes)) => PaneContent::Image {
                src: ImageSource::png(&bytes),
                size: ImageSize {
                    width: *width,
                    height: *height,
                },
            },
            (ClipKind::Image { width, height, .. }, None) => facts(
                Icon::Clipboard,
                "Image",
                vec![("Size", format!("{width} × {height}"))],
            ),
        },
        Preview::Emoji(emoji) => PaneContent::Emoji {
            glyph: emoji.glyph.clone(),
            name: emoji.name.clone(),
        },
        Preview::Web(web) => PaneContent::Web {
            host: web.engine.clone(),
            url: web.url.clone(),
        },
    }
}

/// The room a PDF page gets in the pane: the pane's width, and the height the slot's media box
/// leaves above the name and the four or five rows of facts. `PdfThumb` draws at a fixed size, so
/// a taller page would be clipped by the box.
const PAGE_ROOM: Size = Size {
    width: Px(328.0),
    height: Px(168.0),
};

/// A peeked file as the viewer's pane draws it, in the slot: the picture or lines or page, the
/// file's name and its facts are the pane's own.
fn peeked_slot(peeked: &Arc<AnyPeeked>) -> PaneContent {
    PaneContent::Slot {
        label: peeked.name.clone(),
        element: rsx! {
            Pane { peeked: Arc::clone(peeked), page_room: PAGE_ROOM }
        },
    }
}

/// A file that could not be looked at: its name, where it is and why not.
fn unreadable(file: &sill_launcher::FileFacts, fault: PreviewFault) -> PaneContent {
    let glyph = match file.mime.0.as_str() {
        Mime::FOLDER => Icon::Folder,
        _ => Icon::File,
    };
    let problem = match fault {
        PreviewFault::Missing => "It is no longer there.",
        PreviewFault::Unreadable => "It could not be read.",
    };
    facts(
        glyph,
        &file_name(&file.path),
        vec![
            ("Where", folder(&file.path)),
            ("Problem", problem.to_owned()),
        ],
    )
}

fn facts(icon: Icon, title: &str, rows: Vec<(&str, String)>) -> PaneContent {
    PaneContent::Facts {
        icon: IconSource::Glyph(icon),
        title: title.to_owned(),
        rows: rows
            .into_iter()
            .filter(|(_, value)| !value.is_empty())
            .map(|(label, value)| (label.to_owned(), value))
            .collect(),
    }
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// The folder as the rows word it: `~/…` under the home folder.
fn folder(path: &Path) -> String {
    let parent = path.parent().map(Path::to_path_buf).unwrap_or_default();
    let home = dirs::home_dir();
    match home.and_then(|home| parent.strip_prefix(&home).ok().map(Path::to_path_buf)) {
        Some(inside) if inside.as_os_str().is_empty() => "~".to_owned(),
        Some(inside) => format!("~/{}", inside.display()),
        None => parent.display().to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::{Load, load_of};
    use crate::launcher::preview::load::{Peeking, PreviewFault};

    #[test]
    fn a_file_still_being_looked_at_is_reading() {
        let cases = [
            (None, Load::Ready),
            (Some(Peeking::Reading), Load::Reading),
            (Some(Peeking::Failed(PreviewFault::Missing)), Load::Failed),
            (
                Some(Peeking::Failed(PreviewFault::Unreadable)),
                Load::Failed,
            ),
        ];
        for (peeking, want) in cases {
            assert_eq!(load_of(peeking.as_ref()), want, "{peeking:?}");
        }
    }
}
