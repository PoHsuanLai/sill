//! Where the bar's thumbnails lie, in the desktop's logical pixels, and the commands that keep
//! the compositor's drop targets equal to them. Pure; the bar's surface measures itself and
//! publishes a [`BarLayout`], the daemon sends [`target_commands`].
//!
//! The tile geometry mirrors quire's `SpacesBar` sheet (`spaces_bar.css`: the bar's top padding,
//! the tile's padding and gap, the 160 x 100 card) because the component reports no per-tile
//! rectangle; a change there needs the numbers here changed with it (FINDINGS `mc2`).

use std::collections::BTreeMap;

use porter_core::SpaceId;

use super::DesktopCommand;
use super::target::Rect;

/// The thumbnail card's width in logical pixels (quire's `.ds-spaces-bar-thumb`).
pub const THUMB_W: u32 = 160;
/// The thumbnail card's height in logical pixels.
pub const THUMB_H: u32 = 100;
/// The bar's top padding (`--s-8`).
const BAR_TOP: i32 = 8;
/// The padding around a card inside its tile (`--s-4`, expanded).
const TILE_PAD: i32 = 4;
/// The gap between tiles (`--s-16`).
const TILE_GAP: i32 = 16;
/// The room an expanded bar needs below its top edge: padding, card, gap, name, padding.
pub const BAR_EXPANDED_PX: u32 = 150;

/// One Space's drop target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BarTile {
    /// The Space.
    pub space: SpaceId,
    /// Its card, in the desktop's logical pixels.
    pub rect: Rect,
}

/// The bar of one output once laid out.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct BarLayout {
    /// The cards, left to right.
    pub tiles: Vec<BarTile>,
    /// The size in device pixels a picture of a Space is asked for at: the card at the output's
    /// scale.
    pub thumb: (u32, u32),
}

/// Every laid-out bar, by output name.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct BarLayouts(pub BTreeMap<String, BarLayout>);

/// The card size in device pixels at `scale` (a hundred and twentieth of a unit, as the host
/// states it: 120 is 1x).
pub fn thumb_size(scale: u32) -> (u32, u32) {
    let at = |logical: u32| (logical * scale).div_ceil(120).max(1);
    (at(THUMB_W), at(THUMB_H))
}

/// The cards of `spaces` in a bar whose box is `bar` in the surface's own coordinates, on a
/// surface whose top-left is `origin` in the desktop's: the row of tiles is centred in the bar.
pub fn tiles_of(origin: (i32, i32), bar: Rect, spaces: &[SpaceId]) -> Vec<BarTile> {
    let tile = i32::try_from(THUMB_W).unwrap_or(0) + 2 * TILE_PAD;
    let count = i32::try_from(spaces.len()).unwrap_or(0);
    let row = count * tile + (count - 1).max(0) * TILE_GAP;
    let width = i32::try_from(bar.2).unwrap_or(0);
    let left = origin.0 + bar.0 + (width - row) / 2;
    let top = origin.1 + bar.1 + BAR_TOP + TILE_PAD;
    spaces
        .iter()
        .zip(0_i32..)
        .map(|(space, at)| BarTile {
            space: space.clone(),
            rect: (
                left + at * (tile + TILE_GAP) + TILE_PAD,
                top,
                THUMB_W,
                THUMB_H,
            ),
        })
        .collect()
}

/// The commands that turn the targets `sent` into `now`: a set for each output whose cards are
/// new or moved, an empty set (which clears) for each output that had cards and has none. Outputs
/// in order of name; equal inputs give none.
pub fn target_commands(sent: &BarLayouts, now: &BarLayouts) -> Vec<DesktopCommand> {
    let tiles = |layouts: &BarLayouts, output: &str| {
        layouts
            .0
            .get(output)
            .map(|layout| layout.tiles.clone())
            .unwrap_or_default()
    };
    let outputs: std::collections::BTreeSet<&String> = sent.0.keys().chain(now.0.keys()).collect();
    outputs
        .into_iter()
        .filter_map(|output| {
            let (old, new) = (tiles(sent, output), tiles(now, output));
            (old != new).then(|| DesktopCommand::SetSpaceTargets {
                output: output.clone(),
                targets: new.into_iter().map(|t| (t.space, t.rect)).collect(),
            })
        })
        .collect()
}
