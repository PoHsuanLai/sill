//! The preview pane (M9 lane d, FINDINGS "M9 freeze" F654): Quick Look for the selected row, a
//! column right of the results inside the launcher card. What it shows is
//! `sill_launcher::preview_for(row)`; its keys are [`keys::step_preview`]; its moments go to
//! quire's D0 driver through [`moment::use_pane_moment`].
//!
//! It is quire's `PreviewPane` (m9-palette, Q292), set beside the results as the palette's
//! `aside`: sill maps a [`Preview`] to quire's `PaneContent` (`content.rs`) and a row's actions
//! to `PaneAction`s with their chords. A PDF page comes from `ds_native::use_pdf_page` (the
//! `pdf-thumb` feature): sill passes the path and never rasterises or caches a page (quire keys
//! its cache on path, mtime, size and scale). The pane never takes the keyboard: `focused` draws
//! the ring on the action the panel's Tab reached. The pane is the future Quick Look app's shared
//! part (design/20 §2.4).

mod content;
pub mod keys;
pub mod load;
pub mod moment;

use dioxus::prelude::*;
use ds::{IconSource, PaneAction, Shortcut, Shown};
use sill_launcher::{Action, Preview};

use crate::surfaces::launcher::chord_keys::action_shortcut;

/// The pane's element id (for the blur region, if the card's does not cover it).
pub const PREVIEW_ID: &str = "launcher-preview";

/// The pane's width beside the 680 px card (quire's `ASIDE_WIDTH`; Spotlight's is about 45 % of
/// its window). The panel widens to fit it while the pane is shown (`layer::PANEL_ROOM_WIDTH`).
pub const PREVIEW_WIDTH_PX: u32 = 360;

/// The pane for `preview`, with the selected row's `actions` as its controls (their chords
/// shown, quire design/09 H5), `focused` the control with the keyboard (Tab order). The
/// launcher's panel draws [`LauncherPane`], which also takes the row's icon, the pane's showing
/// and what a click on a control does.
#[component]
pub fn PreviewPane(preview: Preview, actions: Vec<Action>, focused: Option<usize>) -> Element {
    rsx! {
        LauncherPane { preview, actions, focused }
    }
}

/// [`PreviewPane`] as the panel draws it: `icon` is the row's (an app's), `shown` plays the
/// pane in and out (quire calls `on_hidden` when it has left), `onaction` hears a click on a
/// control.
#[component]
pub fn LauncherPane(
    preview: Preview,
    actions: Vec<Action>,
    focused: Option<usize>,
    #[props(default)] icon: Option<IconSource>,
    #[props(default = Shown::Visible)] shown: Shown,
    #[props(default)] on_hidden: EventHandler<()>,
    #[props(default)] onaction: EventHandler<usize>,
) -> Element {
    let content = content::use_content(&preview, icon);
    let actions = pane_actions(&actions);
    rsx! {
        div { id: PREVIEW_ID, class: "sill-launcher-preview",
            ds::PreviewPane {
                content,
                actions,
                focused,
                onaction,
                shown,
                on_hidden,
            }
        }
    }
}

/// A row's actions as the pane's, each with its chord (none: no keys drawn).
pub fn pane_actions(actions: &[Action]) -> Vec<PaneAction> {
    actions
        .iter()
        .map(|action| PaneAction {
            label: action.label.clone(),
            shortcut: action_shortcut(&action.keys).unwrap_or(Shortcut(Vec::new())),
        })
        .collect()
}
