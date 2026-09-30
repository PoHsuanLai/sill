//! The widget surfaces: the desktop widget layer (`desktop`) and the Edit Widgets gallery
//! (`edit`). The providers they draw come from `sill_ui_kit::widgets`.

pub mod desktop;
pub mod edit;

pub use desktop::DesktopWidgets;
pub use edit::EditWidgets;
