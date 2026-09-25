//! A tile's icon (design/08-ICONS.md §4): quire's shipped icon when the entry is one of the apps
//! it stands for (`dock.shipped_icons`), else the app's own themed icon on quire's neutral plate
//! (`Symbolic` for a freedesktop `*-symbolic` file, recoloured like a glyph; `Image` for the
//! rest), quire's `Window` glyph when the app has none (§4.3), `Trash` for the Trash tile.
//!
//! The plate is quire's (`IconView { plate }`), which draws a glyph at its 56 % and an image at
//! its 72 % (`--plate-glyph`, `--plate-inset`); an icon file is still sized at its 72 % share in
//! whole pixels through `IconSize::Px`, at every magnified size. A shipped icon is its own
//! plate (the file carries plate, bevel and shadow at the tile's full size), so it is drawn
//! bare over the whole tile.
//!
//! Under Muted and Monochrome both kinds of raster go through the icon cache (`app_icons`);
//! each is loaded at the largest size the tile is drawn at (magnified, when magnification is
//! on) times the output scale, so a magnification sweep never swaps files mid-flight.

use std::path::{Path, PathBuf};

use ds::{ExternalIcon, Icon, IconPx, IconSize, IconSource, IconUrl};
use sill_services::{App, AppIcon};
use sill_settings::{DockSettings, Magnification, ShippedIcons};

pub use crate::app_icons::IconContext;
use crate::app_icons::{
    DEFAULT_TABLE, LogicalPx, ShippedApp, pick, shipped_app, shipped_art, theme_art,
};
use crate::surfaces::dock::items::ItemKind;

/// design/08 §2.2: the plate is 80.5 % of the tile (824 / 1024).
pub const PLATE_PERMILLE: i32 = 805;

/// design/08 §4.1: a third-party icon fills 72 % of the plate.
pub const ICON_PERMILLE: i32 = 720;

/// What a tile shows, before sizes and styles.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TileArt {
    /// A quire glyph on the plate.
    Glyph(Icon),
    /// A symbolic theme icon on the plate, in the plate's ink.
    Symbolic(PathBuf),
    /// A theme (or the entry's own) icon on the plate.
    Themed(PathBuf),
    /// quire's own icon, which is its own plate.
    Shipped(ShippedApp),
}

/// What the tile for `kind` shows, `app` being its entry when the index has one.
pub fn tile_art(kind: &ItemKind, app: Option<&App>, shipped: ShippedIcons) -> TileArt {
    if *kind == ItemKind::Trash {
        return TileArt::Glyph(Icon::Trash);
    }
    let Some(app) = app else {
        return TileArt::Glyph(Icon::Window);
    };
    if let Some(ours) = shipped_app(&app.id, shipped, DEFAULT_TABLE) {
        return TileArt::Shipped(ours);
    }
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

/// A tile as drawn: on quire's plate, or bare over the whole tile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Drawn {
    Plated(IconSource),
    Bare(IconSource),
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

    /// The plate's side now.
    pub fn plate(self) -> f64 {
        share(self.side, PLATE_PERMILLE)
    }
}

/// `art` drawn at `sizes` in `cx`.
pub fn drawn(art: TileArt, sizes: TileSizes, cx: &IconContext) -> Drawn {
    let plate_icon = |side: f64| share(share(side, PLATE_PERMILLE), ICON_PERMILLE);
    match art {
        TileArt::Glyph(icon) => Drawn::Plated(IconSource::Glyph(icon)),
        TileArt::Symbolic(path) => Drawn::Plated(
            external(&path, plate_icon(sizes.side))
                .map_or(IconSource::Glyph(Icon::Window), IconSource::Symbolic),
        ),
        TileArt::Themed(path) => {
            let px = pick(logical(plate_icon(sizes.largest)), cx.scale);
            let shown = cx.resolver.path(&theme_art(&path, px, cx.look));
            Drawn::Plated(image(&shown, plate_icon(sizes.side)))
        }
        TileArt::Shipped(app) => {
            // The magnified size (when magnification is on), so a magnification sweep never
            // swaps files mid-flight (sill FINDINGS "Icon style").
            let px = pick(logical(sizes.largest), cx.scale);
            match cx.resolver.shipped_path(app, px, cx.look.style) {
                Some(source) => {
                    let shown = cx.resolver.path(&shipped_art(source, app, px, cx.look));
                    Drawn::Bare(image(&shown, sizes.side))
                }
                None => Drawn::Plated(IconSource::Glyph(Icon::Window)),
            }
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

/// `permille` of `side`.
fn share(side: f64, permille: i32) -> f64 {
    side * f64::from(permille) / 1000.0
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
