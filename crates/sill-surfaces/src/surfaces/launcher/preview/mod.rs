//! The preview pane (M9 lane d, FINDINGS "M9 freeze" F654): Quick Look for the selected row, a
//! column right of the results inside the launcher card. What it shows is
//! `sill_launcher::preview_for(row)`; its keys are [`keys::step_preview`]; its moments go to
//! quire's D0 driver through [`moment::use_pane_moment`].
//!
//! It is drawn with quire parts only (Q290-Q297). A PDF is `ds_native::PdfFileThumb { path,
//! size, label }` (quire m9-parts, the `pdf-thumb` feature): sill passes the path and never
//! rasterises or caches a page (quire keys its cache on path, mtime, size and scale). The pane
//! is the future Quick Look app's shared part (design/20 §2.4).
//!
//! **Swap point (Q292).** Until quire's `PreviewPane { content: PaneContent, actions, focused,
//! onaction }` and `CommandPalette { aside }` land, [`PreviewPane`] is a stand-in built from
//! existing quire parts: a `Surface` card in the launcher's material beside the palette's card,
//! an `IconView` for an image or an app, the PDF thumbnail, the excerpt in quire's data face,
//! and each action as a `Button` with its chord's `Kbd`, the focused one drawn as the primary
//! button (the pane's keyboard focus is an index, as quire's pane will take it). Everything in
//! `content.rs` is replaced by one `PaneContent` then.

mod content;
pub mod keys;
pub mod load;
pub mod moment;

use dioxus::prelude::*;
use ds::{Button, ButtonVariant, Corner, Kbd, KbdSize, Material, Px, RootChrome, Surface};
use sill_launcher::{Action, Preview};

use crate::surfaces::launcher::chord_keys::action_shortcut;
use crate::surfaces::launcher::layer::CARD_RADIUS;

/// The pane's element id (for the blur region, if the card's does not cover it).
pub const PREVIEW_ID: &str = "launcher-preview";

/// The pane's width beside the 540 px results column (proposed: Spotlight's is about 45 % of
/// its window). The panel widens to fit it while the pane is shown.
pub const PREVIEW_WIDTH_PX: u32 = 360;

/// The pane's height while the quire pane is missing (the card beside it is as tall as its
/// rows; the stand-in is one fixed card).
pub const PREVIEW_HEIGHT_PX: u32 = 400;

/// What the panel gives the pane beside its props (a context, so the frozen props stay): the
/// launcher's material, and what a press on control `n` does.
#[derive(Clone, Copy)]
pub struct PaneHost {
    pub material: Material,
    pub onaction: Callback<usize>,
}

impl std::fmt::Debug for PaneHost {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PaneHost")
            .field("material", &self.material)
            .finish_non_exhaustive()
    }
}

/// The pane for `preview`, with the selected row's `actions` as its controls (their chords
/// shown, quire design/09 H5), `focused` the control with the keyboard (Tab order).
#[component]
pub fn PreviewPane(preview: Preview, actions: Vec<Action>, focused: Option<usize>) -> Element {
    let host = try_use_context::<PaneHost>();
    let material = host.as_ref().map_or(Material::Sheet, |host| host.material);
    let body = content::use_content(&preview);
    let controls = actions.iter().enumerate().map(|(index, action)| {
        let variant = match focused == Some(index) {
            true => ButtonVariant::Primary,
            false => ButtonVariant::Secondary,
        };
        let shortcut = action_shortcut(&action.keys);
        let onaction = host.as_ref().map(|host| host.onaction);
        rsx! {
            div { key: "{index}", class: "sill-launcher-preview-action",
                Button {
                    variant,
                    label: action.label.clone(),
                    onclick: move |_| {
                        if let Some(onaction) = onaction {
                            onaction.call(index);
                        }
                    },
                }
                if let Some(shortcut) = shortcut {
                    Kbd { shortcut, size: KbdSize::Small }
                }
            }
        }
    });
    rsx! {
        div {
            id: PREVIEW_ID,
            class: "sill-launcher-preview",
            style: "width:{PREVIEW_WIDTH_PX}px;height:{PREVIEW_HEIGHT_PX}px",
            Surface {
                material,
                radius: Some(Corner::Squircle(Px(CARD_RADIUS))),
                chrome: Some(RootChrome::Painted),
                div { class: "sill-launcher-preview-card",
                    {body}
                    div { class: "sill-launcher-preview-actions", {controls} }
                }
            }
        }
    }
}
