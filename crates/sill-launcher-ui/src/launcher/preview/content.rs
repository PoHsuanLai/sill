//! A [`Preview`] as quire's `PaneContent`, with what it names on disk loaded off the UI thread: a
//! file's peek through `load.rs` (the viewer's `anyview_peek`, drawn by its `Card` in quire's
//! slot so one `PreviewPane` keeps its actions, its cue, its pending look and its entrance
//! whatever it shows), a clipboard image's bytes from the clipboard service.

use std::path::Path;
use std::sync::Arc;

use anyview_peek::{AnyPeeked, Card};
use companion_wire::{AnswerBody, footer_line};
use dioxus::prelude::*;
use ds::components::companion::answer::view::AnswerCard;
use ds::components::content::image_source::{ImageSize, ImageSource};
use ds::components::lists::preview::content::{PaneContent, PaneMono};
use ds::prelude::*;
use sill_launcher::{ClipKind, Mime, Preview};
use sill_model::companion::{Companion, RunFacts, memory_line, run_state_of, waiting_line};
use sill_services::CompanionService;
use sill_ui_kit::companion::{ViewCx, answer_view, without_actions};
use sill_ui_kit::hooks::use_service;

use crate::launcher::helpers::HelperCardPane;
use crate::launcher::preview::answer_text::answer_text;
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
    let companion = use_service::<CompanionService>();
    let load = load_of(peeking.as_ref());
    let content = content_of(
        preview,
        icon,
        Loads { peeking, clip_png },
        &companion.read(),
    );
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

fn content_of(
    preview: &Preview,
    icon: Option<IconSource>,
    loads: Loads,
    companion: &Companion,
) -> PaneContent {
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
        Preview::Answer(task) => answer_card(task, companion),
        Preview::Helper(helper) => helper_card(Some(helper), companion),
        Preview::NewHelper => helper_card(None, companion),
        Preview::Run(run) => run_facts(run, companion),
        Preview::Memory(item) => memory_facts(item, companion),
        // An app's own account of a thing: its preview is read on demand, and says nothing
        // until it has.
        Preview::Intent(_) => facts(Icon::Info, "No preview", Vec::new()),
    }
}

/// An answer's card in the pane's slot: its body and footer (the pane lists the row's actions).
fn answer_card(task: &prov::TaskId, companion: &Companion) -> PaneContent {
    let Some(answer) = companion.answers.get(task) else {
        return facts(
            Icon::Sparkles,
            "Assistant",
            vec![("Answer", "Gone".to_owned())],
        );
    };
    let zone = jiff::tz::TimeZone::system();
    let display = |handle: docket_core::Handle| answer.shown.get(&handle).cloned();
    let cx = ViewCx {
        zone: &zone,
        display: &display,
    };
    // A text answer is drawn here, its marks read as emphasis; quire's card draws every other kind.
    let (body, card) = match &answer.wire.body {
        AnswerBody::Text { .. } => (Some(answer_text(answer)), None),
        _ => (
            None,
            Some(without_actions(answer_view(
                &answer.wire,
                &answer.prompt,
                &cx,
            ))),
        ),
    };
    // How the answer was reached, in the words the companion's wire crate prints; no stage, no line.
    let route = footer_line(&answer.routing);
    PaneContent::Slot {
        label: answer.prompt.clone(),
        element: rsx! {
            if let Some(view) = card {
                AnswerCard { view, on_action: move |_| {} }
            }
            if let Some(body) = body {
                {body}
            }
            if !route.is_empty() {
                p { class: "sill-route-line", "{route}" }
            }
        },
    }
}

/// A helper's card in the pane's slot, or the blank card of one not made yet.
fn helper_card(helper: Option<&prov::HelperId>, companion: &Companion) -> PaneContent {
    let label = helper
        .and_then(|id| companion.helpers.line(id))
        .map_or_else(
            || "New Helper".to_owned(),
            |line| line.name.as_str().to_owned(),
        );
    let helper = helper.cloned();
    PaneContent::Slot {
        label,
        element: rsx! {
            HelperCardPane { helper }
        },
    }
}

/// A run's facts: what it was asked, where it stands and what it last thought.
fn run_facts(run: &prov::RunId, companion: &Companion) -> PaneContent {
    let Some(RunFacts {
        goal,
        window,
        step,
        state,
        outcome,
        thought,
        ..
    }) = companion.runs.get(run)
    else {
        return facts(Icon::Play, "Run", vec![("Run", "Gone".to_owned())]);
    };
    facts(
        Icon::Play,
        goal,
        vec![
            ("State", run_state_of(*state, *outcome).label().to_owned()),
            ("Step", step.to_string()),
            ("Window", window.clone().unwrap_or_default()),
            ("Thought", thought.clone().unwrap_or_default()),
        ],
    )
}

/// A memory row's facts: who did what, and in which app.
fn memory_facts(item: &almanac_core::MemoryItem, companion: &Companion) -> PaneContent {
    if let almanac_core::MemoryItem::Fact(id) = item {
        return waiting_facts(id, companion);
    }
    let entry = companion.memory.found.iter().find(
        |entry| matches!(item, almanac_core::MemoryItem::Event(event) if *event == entry.event),
    );
    match entry {
        Some(entry) => {
            let (title, app) = memory_line(entry, &companion.programs);
            facts(
                Icon::Clock,
                &title,
                vec![
                    ("App", app),
                    ("Facts from it", entry.derived_facts.0.to_string()),
                ],
            )
        }
        None => facts(Icon::Clock, "Memory", vec![("Row", "Gone".to_owned())]),
    }
}

/// What an assistant asked to remember: the words, and who asked. Keep and Don't Keep are the
/// row's actions.
fn waiting_facts(id: &almanac_core::FactId, companion: &Companion) -> PaneContent {
    match companion
        .waiting
        .found
        .iter()
        .find(|view| view.fact.id == *id)
    {
        Some(view) => {
            let (text, asked) = waiting_line(view, &companion.programs);
            let who = asked.trim_end_matches(" asked to remember").to_owned();
            facts(Icon::Clock, &text, vec![("Asked by", who)])
        }
        None => facts(Icon::Clock, "Memory", vec![("Row", "Gone".to_owned())]),
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
            Card { peeked: Arc::clone(peeked), page_room: PAGE_ROOM }
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
