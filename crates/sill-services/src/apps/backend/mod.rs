//! The service's effects: what it reaches outside the process.

mod entry;
mod icon_roots;
mod icons;
mod locales;
mod run;
mod scan;

pub use entry::parse_entry;
pub use icon_roots::IconRoots;
pub use icons::{IconTheme, icon_theme, pixmap_dirs, resolve_icon};
pub(crate) use run::run;
pub use scan::scan;
