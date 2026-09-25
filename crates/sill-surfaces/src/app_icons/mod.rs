//! App icons in the icon style (design/08-ICONS.md §2.10-2.11, §4; quire CONSUMING "App icons
//! and the icon style"): quire's shipped icons for the apps they stand for, and every app icon
//! re-coloured for `icons.style` (Colour, Muted, Monochrome) and `icons.monochrome_tint`.
//!
//! Pure: [`shipped`] (which entries take our icons), [`size`] (the raster for a size at a
//! scale), [`look`] (the settings read against the Space), [`key`] and [`plan`].
//! Effects: [`assets`] (finding quire's files), [`render`] (making a raster) and [`cache`]
//! (making each one once, off the UI thread).

pub mod assets;
pub mod cache;
pub mod context;
pub mod key;
pub mod look;
pub mod plan;
pub mod render;
pub mod shipped;
pub mod size;

pub use assets::asset_dir;
pub use cache::{IconResolver, use_icon_resolver};
pub use context::IconContext;
pub use look::{IconLook, icon_look};
pub use plan::{Art, AssetDir, shipped_art, theme_art};
pub use shipped::{DEFAULT_TABLE, ShippedApp, shipped_app};
pub use size::{AssetPx, LogicalPx, pick};
