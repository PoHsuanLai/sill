//! The stand-in pane's content for each kind of preview (swap point Q292: quire's
//! `PaneContent::{Image, Text, Pdf, App, Facts, Emoji, Web}` replaces all of it). Every part is
//! quire's: `PdfFileThumb`, `IconView`, the `ds-mono`, `ds-eyebrow`, `ds-truncate` and
//! `ds-emoji-text` classes; sill lays them out only.

use std::path::{Path, PathBuf};

use dioxus::prelude::*;
use ds::{ExternalIcon, IconPx, IconSize, IconSource, IconUrl, IconView, Px, Size};
use sill_launcher::{ClipKind, Excerpt, Mime, Preview};

use crate::surfaces::launcher::preview::load::{Loaded, use_clip_png, use_excerpt};
use crate::surfaces::launcher::preview::moment::{PaneMoment, use_pane_moment};

/// The PDF page's room in the pane.
const PDF_ROOM: Size = Size {
    width: Px(300.0),
    height: Px(260.0),
};

/// An image's side in the pane (an `IconView` is square).
const IMAGE_SIDE: u8 = 240;

/// The content for `preview`, its loads started (hooks run on every render, whatever the kind).
pub(super) fn use_content(preview: &Preview) -> Element {
    let text_path = match preview {
        Preview::TextFile(path) => Some(path.clone()),
        _ => None,
    };
    let clip_key = match preview {
        Preview::Clipboard(facts) => match facts.kind {
            ClipKind::Image { .. } => Some(facts.key),
            ClipKind::Text { .. } => None,
        },
        _ => None,
    };
    let excerpt = use_excerpt(text_path);
    let clip_png = use_clip_png(clip_key);
    // A slow head read is the pane's `Pending` moment (D0); the pane's Appear, Swap and Leave
    // are the panel's (`panel.rs`), which sees the pane come and go.
    use_pane_moment(match excerpt {
        Some(Loaded::Reading) => PaneMoment::Pending,
        _ => PaneMoment::Still,
    });
    match preview {
        Preview::None => rsx! {},
        Preview::Pdf { path, .. } => rsx! {
            div { class: "sill-launcher-preview-visual",
                ds_native::PdfFileThumb { path: path.clone(), size: PDF_ROOM, label: file_name(path) }
            }
            {facts(&file_name(path).unwrap_or_default(), &[("Kind", "PDF document".to_owned()), ("Where", folder(path))])}
        },
        Preview::Image(path) => rsx! {
            {image(IconUrl::file(path).ok())}
            {facts(&file_name(path).unwrap_or_default(), &[("Where", folder(path))])}
        },
        Preview::TextFile(path) => rsx! {
            {text(excerpt.as_ref())}
            {facts(&file_name(path).unwrap_or_default(), &[("Where", folder(path))])}
        },
        Preview::Text { excerpt } => text(Some(&Loaded::Text(excerpt.clone()))),
        Preview::File(file) => facts(
            &file_name(&file.path).unwrap_or_default(),
            &[
                ("Kind", kind_words(&file.mime)),
                ("Where", folder(&file.path)),
            ],
        ),
        Preview::App { name, detail, .. } => facts(name, &[("About", detail.clone())]),
        Preview::Clipboard(clip) => match &clip.kind {
            ClipKind::Text { excerpt, chars } => rsx! {
                {text(Some(&Loaded::Text(excerpt.clone())))}
                {facts("Clipboard", &[("Length", format!("{} characters", chars.0))])}
            },
            ClipKind::Image { width, height, .. } => rsx! {
                {image(clip_png.as_deref().map(IconUrl::png))}
                {facts("Clipboard", &[("Size", format!("{width} × {height}"))])}
            },
        },
        Preview::Emoji(emoji) => rsx! {
            div { class: "sill-launcher-preview-glyph ds-emoji-text", "{emoji.glyph}" }
            {facts(&emoji.name, &[])}
        },
        Preview::Web(web) => facts(
            "Search the web",
            &[("Engine", web.engine.clone()), ("Address", web.url.clone())],
        ),
    }
}

fn image(url: Option<IconUrl>) -> Element {
    let Some(url) = url else {
        return rsx! {};
    };
    let size = IconSize::Px(IconPx(IMAGE_SIDE));
    rsx! {
        div { class: "sill-launcher-preview-visual",
            IconView { source: IconSource::Image(ExternalIcon { url, size }), size }
        }
    }
}

fn text(loaded: Option<&Loaded>) -> Element {
    let words = match loaded {
        Some(Loaded::Text(Excerpt(text))) if text.is_empty() => "No text to show.".to_owned(),
        Some(Loaded::Text(Excerpt(text))) => text.clone(),
        Some(Loaded::Failed(_)) => "This file could not be read.".to_owned(),
        Some(Loaded::Reading) | None => String::new(),
    };
    rsx! {
        div { class: "sill-launcher-preview-text ds-mono", "{words}" }
    }
}

/// A title and its facts, each a faint label over its value.
fn facts(title: &str, rows: &[(&str, String)]) -> Element {
    rsx! {
        div { class: "sill-launcher-preview-facts",
            div { class: "sill-launcher-preview-title ds-truncate", "{title}" }
            for (label, value) in rows.iter().filter(|(_, v)| !v.is_empty()) {
                div { key: "{label}", class: "sill-launcher-preview-fact",
                    div { class: "ds-eyebrow", "{label}" }
                    div { class: "ds-truncate", "{value}" }
                }
            }
        }
    }
}

fn file_name(path: &Path) -> Option<String> {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
}

/// The folder as the rows word it: `~/…` under the home folder.
fn folder(path: &Path) -> String {
    let parent = path.parent().map(Path::to_path_buf).unwrap_or_default();
    match dirs::home_dir().and_then(|home| parent.strip_prefix(&home).ok().map(PathBuf::from)) {
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
