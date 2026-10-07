//! Edit Widgets (design/28 section 4.1; the reference's widget gallery, "Edit Widgets" at the
//! bottom of Notification Center): quire's `WidgetGallery` in a sheet at the bottom of the
//! output, where the person adds a widget to the desktop or the notification center or takes
//! one away (each widget has one size: the gallery previews and adds it and offers no
//! other). Every choice is a `WidgetEdit` applied through quire's `sill_shell_kit::widget::apply`
//! (`sill_ui_kit::widgets::layout::edited`) and written back as the keys and places sill already keeps
//! (`widgets.center`; the output's own widget list and cells in `desktop-widgets.json`; and
//! `widgets.desktop` turned on by an add to the desktop) through the
//! [`sill_ui_kit::links::EditWidgetsLink`]'s writer the daemon gives it. The sheet lists and edits
//! the desktop widgets of the output it is open on, so each monitor has its own layout.
//! An add is shown by the gallery ("Added" and a check on its button) and by the widget
//! rising on the desktop above the sheet; a removal's words, or a refusal shaken once, are said
//! under the gallery (`status`).
//!
//! One Overlay surface per output, spawned hidden and kept warm like the notification center,
//! following one shared [`sill_ui_kit::links::EditWidgetsLink`]: opened from the center's "Edit
//! Widgets", from the desktop's menu (on the output pressed) and by `sill widgets edit` (on the
//! output of the active window); closed by Done, Escape, a click outside the panel, or `sill
//! widgets edit-close`.

mod io;
mod layer;
mod status;
mod surface;
mod view;

pub use surface::EditWidgets;
pub use view::EDIT_WIDGETS_CSS;
