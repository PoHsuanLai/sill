//! The preview pane (M9 lane d, FINDINGS "M9 freeze" F654): Quick Look for the selected row, a
//! column right of the results inside the launcher card. What it shows is
//! `sill_launcher::preview_for(row)`; its keys are [`keys::step_preview`]; its moments go to
//! quire's D0 driver through [`moment::use_pane_moment`].
//!
//! It is drawn with quire parts only (Q290-Q297): the freeze's component draws nothing, and lane
//! d fills it from quire's preview pane once it lands, never from local markup. A PDF is
//! `ds_native::PdfFileThumb { path, size, label }` (quire m9-parts, behind `ds-native`'s
//! `pdf-thumb` feature, which sill-surfaces enables then): sill passes the path and never
//! rasterises or caches a page (quire keys its cache on path, mtime, size and scale). The pane
//! is the future Quick Look app's shared part (design/20 §2.4).

pub mod keys;
pub mod moment;

use dioxus::prelude::*;
use sill_launcher::{Action, Preview};

/// The pane's element id (for the blur region, if the card's does not cover it).
pub const PREVIEW_ID: &str = "launcher-preview";

/// The pane's width beside the 540 px results column (proposed: Spotlight's is about 45 % of
/// its window). The panel widens to fit it while the pane is shown.
pub const PREVIEW_WIDTH_PX: u32 = 360;

/// The pane for `preview`, with the selected row's `actions` as its controls (their chords
/// shown, quire design/09 H5), `focused` the control with the keyboard (Tab order).
#[component]
pub fn PreviewPane(preview: Preview, actions: Vec<Action>, focused: Option<usize>) -> Element {
    let _ = (&preview, &actions, focused);
    rsx! {}
}
