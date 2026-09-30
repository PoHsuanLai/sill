//! Which raster size an icon is loaded at: its logical size times the output scale, rounded up
//! to a size quire draws. Pure.
//!
//! `ds_settings::app_icon_path` already picks the nearest file to a size a
//! caller asks for, so it could take a raw device-pixel count; [`pick`] stays in front of it
//! anyway (behaviour quire does not cover), because sill still needs to know the bucket itself,
//! not just get a file back: the retint cache key names it, and a dock tile is loaded once for
//! the largest size it is ever drawn at (the magnified size when magnification is on), so a
//! magnification sweep never swaps files mid-flight — [`pick`] is what makes that one, stable
//! bucket rather than a new one every frame. Every one of the 13 sizes is shipped for every app
//! and style, so the bucket [`pick`] names is always an exact match.

use ds::prelude::*;

/// A raster's side in device pixels: one of [`ASSET_SIZES`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct AssetPx(pub u16);

/// An icon's side in logical pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct LogicalPx(pub u16);

/// The sizes quire draws each app icon at, every one directly (not downscaled).
pub const ASSET_SIZES: [u16; 13] = [16, 22, 24, 32, 36, 44, 48, 64, 72, 96, 128, 256, 512];

/// The smallest drawn size covering `logical` at `scale` (so the raster is never stretched up),
/// the largest when none does.
pub fn pick(logical: LogicalPx, scale: Scale) -> AssetPx {
    // Device pixels, rounded up: 48 at 1.75 is 84 exactly, 49 at 1.5 is 73.5 -> 74.
    let device = (u32::from(logical.0) * scale.0).div_ceil(Scale::ONE.0);
    let largest = ASSET_SIZES[ASSET_SIZES.len() - 1];
    AssetPx(
        ASSET_SIZES
            .into_iter()
            .find(|&size| u32::from(size) >= device)
            .unwrap_or(largest),
    )
}

#[cfg(test)]
mod tests {
    use super::{AssetPx, LogicalPx, pick};
    use ds::prelude::*;

    #[test]
    fn a_tile_takes_the_drawn_size_that_covers_it() {
        // (logical, scale in 120ths, file)
        const CASES: &[(u16, u32, u16)] = &[
            (48, 120, 48),
            (48, 180, 72),
            (48, 240, 96),
            (48, 210, 96),
            (48, 150, 64),
            (24, 240, 48),
            (28, 120, 32),
            (49, 180, 96),
            (96, 240, 256),
            (300, 240, 512),
            (1, 120, 16),
        ];
        for &(logical, scale, want) in CASES {
            assert_eq!(
                pick(LogicalPx(logical), Scale(scale)),
                AssetPx(want),
                "{logical} at {scale}/120"
            );
        }
    }
}
