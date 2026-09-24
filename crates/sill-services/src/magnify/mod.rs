//! The dock's magnification: the Plank parabola (design/10 §10.5.1) and the pointer-anchored
//! layout (§10.5.2). Pure; `sill-surfaces::surfaces::dock` calls it once per frame with the
//! latest pointer x and the eased progress.

mod layout;
mod zoom;

pub use layout::{Layout, TileBox, magnify, rest_centers};
pub use zoom::{sizes, zoom};

use serde::{Deserialize, Serialize};

use crate::units::Cpx;

/// The geometry the curve and the layout need, from `DockSettings` (`tile_size_px`,
/// `magnified_size_px`, `influence_radius_px`, `tile_gap_px`, `edge_clamp_px`), the pill
/// padding and the output width.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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
    /// design/10 §10.3.1 at rest: `T` 48, `S` 96, `R` 96, `g` 4, `pad` 8, clamp 8.
    pub fn design_defaults(output_width: Cpx) -> MagnifyParams {
        MagnifyParams {
            tile: Cpx::px(48),
            magnified: Cpx::px(96),
            radius: Cpx::px(96),
            gap: Cpx::px(4),
            pad: Cpx::px(8),
            edge_clamp: Cpx::px(8),
            output_width,
        }
    }
}
