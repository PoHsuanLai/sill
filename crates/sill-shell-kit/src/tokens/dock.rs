//! The dock's geometry tokens (the macOS polish pass, 2026-09-24; design/10-BEHAVIOUR-dock.md
//! sections 10.3.1 and 10.3.2): the tile, the gap between tiles, the pill's padding, the running
//! dot, the pill's corner, the edge clamp and the separator, each a tuned token a look or the
//! person's style sheet moves; only the tile's size has a settings key (macOS's Size slider).
//! The floor's own token, `--dock-floor`, is quire's `DockFloorToken`, because quire's stylesheet
//! draws `.ds-dock-floor`. The shell lays the dock out itself; these are the numbers it and its
//! dock pieces (`RunningDot`, `DockFloor`, an `IconView` plate) share.

use ds::base::geometry::units::Px;
use ds::base::word::Word;
use ds::style::tokens::dock_floor::DockFloorToken;
use ds::style::tokens::token::Token;

/// One dock geometry token, each a tuned token (a look or the person's style sheet may write its
/// `input`, the default behind it). Rust that computes with one reads it from the document
/// (`sill_dock::dock::tokens`), with these defaults as its constants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Word, Token)]
#[token(prefix = "", kind = tuned)]
pub enum DockToken {
    /// `--dock-tile`: a tile at rest (`dock.tile_size_px`, 48; macOS `tilesize` 48, H).
    #[token(name = "dock-tile", input = "--dock-tile-px", value = "48px")]
    Tile,
    /// `--dock-gap`: between two tiles (8).
    #[token(name = "dock-gap", input = "--dock-gap-px", value = "8px")]
    Gap,
    /// `--dock-pad`: the pill's padding on every side (6).
    #[token(name = "dock-pad", input = "--dock-pad-px", value = "6px")]
    Pad,
    /// `--dock-dot`: the running dot's diameter (4).
    #[token(name = "dock-dot", input = "--dock-dot-px", value = "4px")]
    Dot,
    /// `--dock-dot-gap`: from the tile's bottom edge to the dot's centre (3), inside the pill's
    /// 6 px padding.
    #[token(name = "dock-dot-gap", input = "--dock-dot-gap-px", value = "3px")]
    DotGap,
    /// `--sill-dock-pill-radius`: the pill's squircle corner (22).
    #[token(
        name = "sill-dock-pill-radius",
        input = "--sill-dock-pill-radius-px",
        value = "22px"
    )]
    PillRadius,
    /// `--sill-dock-edge-clamp`: how close the pill gets to the output's sides (8).
    #[token(
        name = "sill-dock-edge-clamp",
        input = "--sill-dock-edge-clamp-px",
        value = "8px"
    )]
    EdgeClamp,
    /// `--sill-dock-separator-height`: the line before Trash (36).
    #[token(
        name = "sill-dock-separator-height",
        input = "--sill-dock-separator-height-px",
        value = "36px"
    )]
    SeparatorHeight,
}

/// Whether the dock draws its reflective floor (`dock.floor`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum DockFloorSetting {
    /// No floor: the pill alone (the default).
    #[default]
    Off,
    /// A soft light band along the pill's floor, under the tiles.
    On,
}

/// What the settings write around the dock: the tile's size (`dock.tile_size_px`) and the floor
/// (`dock.floor`), as the tokens' inputs on any element around the dock.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DockMetrics {
    /// A tile at rest, 48.
    pub tile: Px,
    /// The reflective floor, off.
    pub floor: DockFloorSetting,
}

impl Default for DockMetrics {
    /// The keys' defaults (settled 2026-09-24, design/10 sections 10.3.1 and 10.3.2).
    fn default() -> Self {
        DockMetrics {
            tile: Px(48.0),
            floor: DockFloorSetting::Off,
        }
    }
}

impl DockMetrics {
    /// Every input, inline: `--dock-tile-px:48px;--dock-floor-on:0;`.
    pub fn style_attr(&self) -> String {
        let tile = format!("{}px", self.tile.0.round().clamp(0.0, 999.0) as u16);
        let floor = match self.floor {
            DockFloorSetting::Off => "0",
            DockFloorSetting::On => "1",
        };
        [
            DockToken::Tile.write(&tile),
            DockFloorToken::Floor.write(floor),
        ]
        .concat()
    }
}

#[cfg(test)]
mod tests {
    use super::{DockFloorSetting, DockMetrics, DockToken};
    use ds::style::tokens::token::TokenScope;

    #[test]
    fn the_defaults_write_what_the_stylesheet_falls_back_to() {
        let written = DockMetrics::default().style_attr();
        let want = DockToken::Tile.write(DockToken::Tile.fallback(TokenScope::BASE));
        assert!(written.contains(&want), "{want} not in {written}");
    }

    #[test]
    fn the_floor_is_one_inline_switch() {
        let on = DockMetrics {
            floor: DockFloorSetting::On,
            ..DockMetrics::default()
        };
        assert!(on.style_attr().contains("--dock-floor-on:1;"));
    }
}
