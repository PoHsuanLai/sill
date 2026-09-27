//! Edit Widgets (design/28 section 4.1; the reference's widget gallery, "Edit Widgets" at the
//! bottom of Notification Center): quire's `WidgetGallery` in a panel over the output, where the
//! person adds a widget to the desktop or the notification center, changes a placed one's size,
//! or takes it away. Every choice is a `WidgetEdit` applied through quire's
//! `ds::widget::apply` (`crate::widgets::layout::edited`) and written back as the keys and
//! places sill already keeps (`widgets.center`, `widgets.desktop_widgets`, `widgets.sizes`,
//! `desktop-widgets.json`) through the [`WidgetsWriter`] the daemon gives the root.
//!
//! One Overlay surface per output, spawned hidden and kept warm like the notification center,
//! following one shared [`EditWidgetsLink`]: opened from the center's "Edit Widgets" and by
//! `sill widgets edit`; closed by Done, Escape, a click outside the panel, or `sill widgets
//! edit-close`. The desktop has no context menu yet, so there is no desktop entry to it.

pub mod layer;
pub mod link;
pub mod writer;

mod drive;
mod view;

pub use layer::{EDIT_PANEL_ID, edit_widgets_layer};
pub use link::{EditShown, EditWidgetsLink};
pub use view::EDIT_WIDGETS_CSS;
pub use writer::{WidgetsWrite, WidgetsWriter};

use dioxus::prelude::*;
use shell_host::OutputInfo;

/// Edit Widgets on `output`. Needs an [`EditWidgetsLink`] root context (the daemon's
/// `surface_root` gives every surface one) and, to change anything, a [`WidgetsWriter`].
#[component]
pub fn EditWidgets(output: OutputInfo) -> Element {
    view::edit(output)
}
