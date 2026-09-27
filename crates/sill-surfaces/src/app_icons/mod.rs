//! App icons in the icon style (design/08-ICONS.md §4; quire CONSUMING "App icons and the icon
//! style"): every app's own (hicolor, or the entry's own file) icon on quire's neutral plate,
//! re-coloured for `icons.style` (Colour, Muted, Monochrome) and `icons.monochrome_tint`. sill
//! no longer ships its own per-app artwork (FINDINGS "App icons: shipped set removed"): every
//! entry takes this one fallback path.
//!
//! Pure: [`size`] (the raster for a size at a scale), [`look`] (the settings read against the
//! Space), [`key`] and [`plan`]. Effects: [`render`] (making a raster) and [`cache`] (making a
//! re-tinted raster once, off the UI thread).

pub mod cache;
pub mod context;
pub mod key;
pub mod look;
pub mod plan;
pub mod render;
pub mod size;

pub use cache::{IconResolver, use_icon_resolver};
pub use context::IconContext;
pub use look::{IconLook, icon_look};
pub use plan::{Art, theme_art};
pub use size::{AssetPx, LogicalPx, pick};
