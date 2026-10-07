//! The dock's geometry tokens (the macOS polish pass, 2026-09-24; design/10-BEHAVIOUR-dock.md
//! sections 10.3.1 and 10.3.2): the tile, the gap between tiles, the pill's padding, the running
//! dot and the switch of the optional reflective floor, each a tuned token its settings key moves.
//! The floor's own token, `--dock-floor`, is quire's `DockFloorToken`, because quire's stylesheet
//! draws `.ds-dock-floor`. The shell lays the dock out itself; these are the numbers it and its
//! dock pieces (`RunningDot`, `DockFloor`, an `IconView` plate) share.

use ds::base::geometry::units::Px;
use ds::base::word::Word;
use ds::style::tokens::dock_floor::DockFloorToken;
use ds::style::tokens::token::Token;
use ds::style::tokens::tuned::px;

/// One dock geometry token, each a tuned token its settings key moves.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Word, Token)]
#[token(prefix = "", kind = tuned)]
pub enum DockToken {
    /// `--dock-tile`: a tile at rest (`dock.tile_size_px`, 48; macOS `tilesize` 48, H).
    #[token(name = "dock-tile", input = "--dock-tile-px", value = "48px")]
    Tile,
    /// `--dock-gap`: between two tiles (`dock.tile_gap_px`, 8).
    #[token(name = "dock-gap", input = "--dock-gap-px", value = "8px")]
    Gap,
    /// `--dock-pad`: the pill's padding on every side (`dock.pill_padding_px`, 6).
    #[token(name = "dock-pad", input = "--dock-pad-px", value = "6px")]
    Pad,
    /// `--dock-dot`: the running dot's diameter (`dock.running_dot_diameter_px`, 4).
    #[token(name = "dock-dot", input = "--dock-dot-px", value = "4px")]
    Dot,
    /// `--dock-dot-gap`: from the tile's bottom edge to the dot's centre (`dock.running_dot_gap_px`,
    /// 3), inside the pill's 6 px padding.
    #[token(name = "dock-dot-gap", input = "--dock-dot-gap-px", value = "3px")]
    DotGap,
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

/// The dock's geometry from the settings, written as the tokens' inputs on any element around
/// the dock.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DockMetrics {
    /// A tile at rest, 48.
    pub tile: Px,
    /// Between two tiles, 8.
    pub gap: Px,
    /// The pill's padding, 6.
    pub pad: Px,
    /// The running dot's diameter, 4.
    pub dot: Px,
    /// From the tile's bottom edge to the dot's centre, 3.
    pub dot_gap: Px,
    /// The reflective floor, off.
    pub floor: DockFloorSetting,
}

impl Default for DockMetrics {
    /// The keys' defaults (settled 2026-09-24, design/10 sections 10.3.1 and 10.3.2).
    fn default() -> Self {
        DockMetrics {
            tile: Px(48.0),
            gap: Px(8.0),
            pad: Px(6.0),
            dot: Px(4.0),
            dot_gap: Px(3.0),
            floor: DockFloorSetting::Off,
        }
    }
}

impl DockMetrics {
    /// Every input, inline: `--dock-tile-px:48px;…`.
    pub fn style_attr(&self) -> String {
        let length = |value: Px| px(value.0.round().clamp(0.0, 999.0) as u16);
        let floor = match self.floor {
            DockFloorSetting::Off => "0",
            DockFloorSetting::On => "1",
        };
        [
            DockToken::Tile.write(&length(self.tile)),
            DockToken::Gap.write(&length(self.gap)),
            DockToken::Pad.write(&length(self.pad)),
            DockToken::Dot.write(&length(self.dot)),
            DockToken::DotGap.write(&length(self.dot_gap)),
            DockFloorToken::Floor.write(floor),
        ]
        .concat()
    }

    /// The pill's height at rest: a tile and the padding above and below it (60 at the
    /// defaults).
    pub fn pill_height(&self) -> Px {
        Px(self.tile.0 + 2.0 * self.pad.0)
    }

    /// The pill's width at rest for `tiles` tiles: `n T + (n-1) g + 2 pad` (design/10 section
    /// 10.3.1).
    pub fn pill_width(&self, tiles: u16) -> Px {
        let n = f32::from(tiles);
        let gaps = f32::from(tiles.saturating_sub(1));
        Px(n * self.tile.0 + gaps * self.gap.0 + 2.0 * self.pad.0)
    }
}

#[cfg(test)]
mod tests {
    use super::{DockFloorSetting, DockMetrics, DockToken};
    use ds::base::geometry::units::Px;
    use ds::base::word::Word;
    use ds::style::tokens::token::TokenScope;

    #[test]
    fn the_defaults_write_what_the_stylesheet_falls_back_to() {
        let written = DockMetrics::default().style_attr();
        for token in DockToken::ALL.iter().copied() {
            let want = token.write(token.fallback(TokenScope::BASE));
            assert!(written.contains(&want), "{want} not in {written}");
        }
    }

    #[test]
    fn the_pill_is_the_tiles_gaps_and_padding() {
        let dock = DockMetrics::default();
        assert_eq!(dock.pill_height(), Px(60.0));
        assert_eq!(dock.pill_width(3), Px(3.0 * 48.0 + 2.0 * 8.0 + 12.0));
        assert_eq!(dock.pill_width(1), Px(60.0));
        let on = DockMetrics {
            floor: DockFloorSetting::On,
            ..dock
        };
        assert!(on.style_attr().contains("--dock-floor-on:1;"));
    }
}
