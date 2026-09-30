//! The launcher's catcher (design/20 §1.3): a transparent Overlay over the whole output, shown
//! with the panel, whose press anywhere outside the panel closes the launcher. It follows the
//! same [`sill_ui_kit::links::LauncherLink`] as the panel.

mod layer;
mod surface;
mod view;

pub use surface::LauncherCatcher;
