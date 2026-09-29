//! Edit Widgets (design/28 section 4.1; the reference's widget gallery, "Edit Widgets" at the
//! bottom of Notification Center): quire's `WidgetGallery` in a sheet at the bottom of the
//! output, where the person adds a widget to the desktop or the notification center or takes
//! one away (each widget has one size: the gallery previews and adds it and offers no
//! other). Every choice is a `WidgetEdit` applied through quire's `ds::widget::apply`
//! (`crate::widgets::layout::edited`) and written back as the keys and places sill already keeps
//! (`widgets.center`, `widgets.desktop_widgets`, `desktop-widgets.json`, and `widgets.desktop`
//! turned on by an add to the desktop) through the [`crate::links::WidgetsWriter`] the daemon gives the root.
//! An add is shown by the gallery ("Added" and a check on its button) and by the widget
//! rising on the desktop above the sheet; a removal's words, or a refusal shaken once, are said
//! under the gallery (`status`).
//!
//! One Overlay surface per output, spawned hidden and kept warm like the notification center,
//! following one shared [`crate::links::EditWidgetsLink`]: opened from the center's "Edit Widgets" and by
//! `sill widgets edit`; closed by Done, Escape, a click outside the panel, or `sill widgets
//! edit-close`. The desktop has no context menu yet, so there is no desktop entry to it.

pub mod layer;

mod drive;
mod status;
mod view;

pub use layer::{EDIT_PANEL_ID, edit_widgets_layer};
pub use view::EDIT_WIDGETS_CSS;

use dioxus::prelude::*;
use shell_host::OutputInfo;

use crate::host_modality::HostModalityBridge;

/// Edit Widgets on `output`. Needs an [`EditWidgetsLink`] root context (the daemon's
/// `surface_root` gives every surface one) and, to change anything, a [`WidgetsWriter`].
#[component]
pub fn EditWidgets(output: OutputInfo) -> Element {
    rsx! {
        HostModalityBridge { {view::edit(output)} }
    }
}
