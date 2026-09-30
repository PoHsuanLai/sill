//! The launcher surfaces: the panel (rows, preview pane, actions menu) and its click catcher,
//! over `sill-launcher`'s core and `sill-search`'s results.

pub mod launcher;

pub use launcher::Launcher;
pub use launcher::LauncherCatcher;
