//! A tile's icon (design/08-ICONS.md §4): the app's own themed icon as it is, on no plate
//! (`Symbolic` for a freedesktop `*-symbolic` file, recoloured like a glyph; `Image` for the
//! rest), quire's `Window` glyph when the app has none (§4.3), `Trash` for the Trash tile. Every
//! entry takes this one path.
//!
//! The icon fills the whole tile, in whole pixels through `IconSize::Px`, at every magnified size
//! (the owner, 2026-10-20: no white plate behind the icons; an app with only an outline icon gets
//! a proper one instead).
//!
//! Under Muted and Monochrome the raster goes through the icon cache (`app_icons`); it is
//! loaded at the largest size the tile is drawn at (magnified, when magnification is on) times
//! the output scale, so a magnification sweep never swaps files mid-flight.

use std::path::{Path, PathBuf};

use ds::prelude::*;
use ds::style::icon::render::IconPx;
use ds::style::icon::url::IconUrl;
use sill_model::apps::{App, AppIcon};
use sill_settings::{DockSettings, Magnification};
use sill_shell_kit::dock::tile::plate_side;

use crate::dock::items::ItemKind;
pub use sill_ui_kit::app_icons::IconContext;
use sill_ui_kit::app_icons::{LogicalPx, pick, theme_art};

/// What a tile shows, before sizes and styles.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TileArt {
    /// A quire glyph.
    Glyph(Icon),
    /// A symbolic theme icon, in the dock's ink.
    Symbolic(PathBuf),
    /// A theme (or the entry's own) icon.
    Themed(PathBuf),
}

/// What the tile for `kind` shows, `app` being its entry when the index has one.
pub fn tile_art(kind: &ItemKind, app: Option<&App>) -> TileArt {
    if *kind == ItemKind::Trash {
        return TileArt::Glyph(Icon::Trash);
    }
    let Some(app) = app else {
        return TileArt::Glyph(Icon::Window);
    };
    match &app.icon {
        AppIcon::File(path) if !path.is_absolute() => TileArt::Glyph(Icon::Window),
        AppIcon::File(path) if symbolic(path) => TileArt::Symbolic(path.clone()),
        AppIcon::File(path) => TileArt::Themed(path.clone()),
        AppIcon::None => TileArt::Glyph(Icon::Window),
    }
}

/// Whether a file is a freedesktop `*-symbolic` icon.
fn symbolic(path: &Path) -> bool {
    path.file_stem()
        .and_then(|stem| stem.to_str())
        .is_some_and(|stem| stem.ends_with("-symbolic"))
}

/// The sizes one frame draws a tile at, in logical pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TileSizes {
    /// The tile's side now.
    pub side: f64,
    /// The largest side it is ever drawn at.
    pub largest: f64,
}

impl TileSizes {
    /// This frame's `side` under `dock`'s sizes.
    pub fn new(side: f64, dock: &DockSettings) -> TileSizes {
        let rest = f64::from(dock.tile_size_px.0);
        let largest = match dock.magnification {
            Magnification::On => rest.max(f64::from(dock.magnified_size_px.0)),
            Magnification::Off => rest,
        };
        TileSizes {
            side,
            largest: largest.max(side),
        }
    }
}

/// `art` drawn at `sizes` in `cx`, filling the tile.
pub fn drawn(art: TileArt, sizes: TileSizes, cx: &IconContext) -> IconSource {
    let plate_icon = |side: f64| f64::from(plate_side(Px(side as f32)).0);
    match art {
        TileArt::Glyph(icon) => IconSource::Glyph(icon),
        TileArt::Symbolic(path) => external(&path, plate_icon(sizes.side))
            .map_or(IconSource::Glyph(Icon::Window), IconSource::Symbolic),
        TileArt::Themed(path) => {
            let px = pick(logical(plate_icon(sizes.largest)), cx.scale);
            let shown = cx.resolver.path(&theme_art(&path, px, cx.look));
            image(&shown, plate_icon(sizes.side))
        }
    }
}

/// `path` as an image icon `side` wide, the window glyph when it is not a file URL.
fn image(path: &Path, side: f64) -> IconSource {
    external(path, side).map_or(IconSource::Glyph(Icon::Window), IconSource::Image)
}

fn external(path: &Path, side: f64) -> Option<ExternalIcon> {
    Some(ExternalIcon {
        url: IconUrl::file(path).ok()?,
        size: whole(side),
    })
}

/// `side` in whole pixels (at least one).
fn whole(side: f64) -> IconSize {
    // Rounded and clamped to u8's range, so the cast is exact.
    IconSize::Px(IconPx(side.round().clamp(1.0, 255.0) as u8))
}

/// `side` as a logical size, rounded.
fn logical(side: f64) -> LogicalPx {
    // Rounded and clamped to u16's range, so the cast is exact.
    LogicalPx(side.round().clamp(1.0, f64::from(u16::MAX)) as u16)
}

#[cfg(test)]
#[path = "icon_tests.rs"]
mod tests;
