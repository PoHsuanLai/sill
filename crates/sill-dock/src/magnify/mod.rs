//! The dock's magnification: the Plank parabola (design/10 §10.5.1) and the pointer-anchored
//! layout (§10.5.2). Pure; `dock` calls it once per frame with the
//! latest pointer x and the eased progress.

mod layout;
mod zoom;

pub use layout::{Layout, TileBox, magnify, rest_centers};
pub use zoom::{sizes, zoom};

use crate::dock::tokens;
use sill_model::units::Cpx;

/// The geometry the curve and the layout need: `DockSettings` (`tile_size_px`,
/// `magnified_size_px`), the influence radius, the `DockTokens` (gap, padding, edge clamp) and
/// the output width.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MagnifyParams {
    /// `T`: a tile at rest.
    pub tile: Cpx,
    /// `S`: the tile under the pointer at full progress; `M = S / T`.
    pub magnified: Cpx,
    /// `R`: distance from a tile centre at which its zoom falls to 1.
    pub radius: Cpx,
    /// `g`: gap between tiles.
    pub gap: Cpx,
    /// `pad`: pill padding on each side.
    pub pad: Cpx,
    /// Distance the pill keeps from both output edges.
    pub edge_clamp: Cpx,
    pub output_width: Cpx,
}

impl MagnifyParams {
    /// design/10 §10.3.1 at rest: `T` 48, `S` 72, `R` 72, `g` 8, `pad` 6, clamp 8 (the macOS
    /// polish pass moved `g` and `pad` from 4 and 8).
    pub fn design_defaults(output_width: Cpx) -> MagnifyParams {
        MagnifyParams {
            tile: Cpx::px(48),
            magnified: Cpx::px(72),
            radius: Cpx::px(72),
            gap: Cpx::px(i32::from(tokens::GAP)),
            pad: Cpx::px(i32::from(tokens::PAD)),
            edge_clamp: Cpx::px(i32::from(tokens::EDGE_CLAMP)),
            output_width,
        }
    }
}
