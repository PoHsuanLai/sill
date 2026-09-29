//! A [`Preview`] as quire's `PaneContent`, with what it names on disk loaded off the UI
//! thread: a PDF's page through `ds_native::use_pdf_page` (asked only while the preview is a
//! PDF, so one pane stays mounted whatever it shows), a text file's head and an image's size
//! through `load.rs`, a clipboard image's bytes from the clipboard service.

use std::path::Path;

use ds::{Icon, IconSource, ImageSize, ImageSource, PANE_MEDIA, PaneContent, PaneMono, PdfPage};
use sill_launcher::{ClipKind, Excerpt, Mime, Preview};

use crate::surfaces::launcher::preview::load::{Loaded, use_clip_png, use_excerpt, use_image_size};

/// How far what a preview names has loaded.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) enum Load {
    /// All of it, or it needs no load.
    Ready,
    /// A text file's head or a PDF's page is still being read.
    Reading,
    /// It could not be read.
    Failed,
}

/// The content for `preview` (`icon`: the row's, for an app), and how far it has loaded. Hooks
/// run on every render, whatever the kind.
pub(super) fn use_content(preview: &Preview, icon: Option<IconSource>) -> (PaneContent, Load) {
    let pdf = match preview {
        Preview::Pdf { path, .. } => Some(path.clone()),
        _ => None,
    };
    let text = match preview {
        Preview::TextFile(path) => Some(path.clone()),
        _ => None,
    };
    let image = match preview {
        Preview::Image(path) => Some(path.clone()),
        _ => None,
    };
    let clip = match preview {
        Preview::Clipboard(facts) => match facts.kind {
            ClipKind::Image { .. } => Some(facts.key),
            ClipKind::Text { .. } => None,
        },
        _ => None,
    };
    let page = ds_native::use_pdf_page(pdf, PANE_MEDIA);
    let excerpt = use_excerpt(text);
    let size = use_image_size(image);
    let clip_png = use_clip_png(clip);
    let load = load_of(excerpt.as_ref(), page.as_ref());
    let content = content_of(
        preview,
        icon,
        Loads {
            page,
            excerpt,
            size,
            clip_png,
        },
    );
    (content, load)
}

/// How far a text file's head and a PDF's page have loaded (`None`: the preview has none).
fn load_of(excerpt: Option<&Loaded>, page: Option<&PdfPage>) -> Load {
    match (excerpt, page) {
        (Some(Loaded::Reading), _) | (_, Some(PdfPage::Loading)) => Load::Reading,
        (Some(Loaded::Failed(_)), _) | (_, Some(PdfPage::Failed(_))) => Load::Failed,
        (Some(Loaded::Text(_)) | None, Some(PdfPage::Ready { .. } | PdfPage::Empty) | None) => {
            Load::Ready
        }
    }
}

/// What the hooks brought for the preview.
struct Loads {
    page: Option<PdfPage>,
    excerpt: Option<Loaded>,
    size: Option<ImageSize>,
    clip_png: Option<Vec<u8>>,
}

fn content_of(preview: &Preview, icon: Option<IconSource>, loads: Loads) -> PaneContent {
    match preview {
        Preview::None => facts(Icon::Search, "No preview", Vec::new()),
        Preview::Pdf { path, .. } => PaneContent::Pdf {
            page: loads.page.unwrap_or(PdfPage::Loading),
            name: file_name(path),
        },
        Preview::Image(path) => match ImageSource::file(path) {
            Ok(src) => PaneContent::Image {
                src,
                size: loads.size.unwrap_or(ImageSize {
                    width: 1,
                    height: 1,
                }),
            },
            Err(_) => facts(Icon::File, &file_name(path), vec![("Where", folder(path))]),
        },
        Preview::TextFile(_) => PaneContent::Text {
            excerpt: text_of(loads.excerpt.as_ref()),
            mono: PaneMono::Monospace,
        },
        Preview::Text { excerpt } => PaneContent::Text {
            excerpt: excerpt.0.clone(),
            mono: PaneMono::Proportional,
        },
        Preview::File(file) => {
            let glyph = match file.mime.0.as_str() {
                Mime::FOLDER => Icon::Folder,
                _ => Icon::File,
            };
            facts(
                glyph,
                &file_name(&file.path),
                vec![
                    ("Kind", kind_words(&file.mime)),
                    ("Where", folder(&file.path)),
                ],
            )
        }
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

fn text_of(loaded: Option<&Loaded>) -> String {
    match loaded {
        Some(Loaded::Text(Excerpt(text))) if text.is_empty() => "No text to show.".to_owned(),
        Some(Loaded::Text(Excerpt(text))) => text.clone(),
        Some(Loaded::Failed(_)) => "This file could not be read.".to_owned(),
        Some(Loaded::Reading) | None => String::new(),
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

fn kind_words(mime: &Mime) -> String {
    match mime.0.as_str() {
        Mime::FOLDER => "Folder".to_owned(),
        other => other.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::{Load, load_of};
    use crate::surfaces::launcher::preview::load::Loaded;
    use ds::{PdfPage, PdfTrouble};
    use sill_launcher::Excerpt;
    use sill_services::preview::PreviewFault;

    #[test]
    fn a_slow_head_or_page_is_reading() {
        let text = Loaded::Text(Excerpt("hi".to_owned()));
        let reading = Loaded::Reading;
        let failed = Loaded::Failed(PreviewFault::Unreadable);
        let locked = PdfPage::Failed(PdfTrouble::Locked);
        let cases: [(Option<&Loaded>, Option<&PdfPage>, Load); 7] = [
            (None, None, Load::Ready),
            (Some(&text), None, Load::Ready),
            (Some(&reading), None, Load::Reading),
            (Some(&failed), None, Load::Failed),
            (None, Some(&PdfPage::Loading), Load::Reading),
            (None, Some(&PdfPage::Empty), Load::Ready),
            (None, Some(&locked), Load::Failed),
        ];
        for (excerpt, page, want) in cases {
            assert_eq!(load_of(excerpt, page), want, "{excerpt:?} {page:?}");
        }
    }
}
