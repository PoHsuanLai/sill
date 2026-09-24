//! The dock's settings as `sill_services::magnify`'s parameters.

use sill_services::{Cpx, MagnifyParams};
use sill_settings::DockSettings;

use crate::surfaces::dock::layer::PAD_PX;

/// `MagnifyParams` for `dock` on an output `output_width_px` wide.
pub fn magnify_params(dock: &DockSettings, output_width_px: i32) -> MagnifyParams {
    let px = |v: u16| Cpx::px(i32::from(v));
    MagnifyParams {
        tile: px(dock.tile_size_px.0),
        magnified: px(dock.magnified_size_px.0),
        radius: px(dock.influence_radius_px.0),
        gap: px(dock.tile_gap_px.0),
        pad: Cpx::px(i32::try_from(PAD_PX).unwrap_or(8)),
        edge_clamp: px(dock.edge_clamp_px.0),
        output_width: Cpx::px(output_width_px),
    }
}

#[cfg(test)]
mod tests {
    use super::magnify_params;
    use sill_services::{Cpx, MagnifyParams};
    use sill_settings::DockSettings;

    #[test]
    fn default_settings_are_the_design_geometry() {
        assert_eq!(
            magnify_params(&DockSettings::default(), 1920),
            MagnifyParams::design_defaults(Cpx::px(1920))
        );
    }
}
