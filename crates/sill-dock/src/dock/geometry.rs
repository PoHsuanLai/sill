//! The dock's settings as `crate::magnify`'s parameters and as quire's `DockMetrics`
//! (the macOS polish pass: tile 48, gap 8, pad 6, dot 4 centred 3 under the tile, floor off).

use crate::magnify::MagnifyParams;
use ds::prelude::*;
use ds_shell::tokens::dock::{DockFloorSetting, DockMetrics};
use ds_style::tokens::shape::Corner;
use sill_model::units::Cpx;
use sill_settings::{DockFloor, DockSettings};

/// quire's dock tokens from `dock.tile_size_px`, `dock.tile_gap_px`, `dock.pill_padding_px`,
/// `dock.running_dot_diameter_px`, `dock.running_dot_gap_px` and `dock.floor`: written inline
/// around the dock's root (`DockMetrics::style_attr`), where `RunningDot` and `DockFloor` read
/// them.
pub fn dock_metrics(dock: &DockSettings) -> DockMetrics {
    let px = |value: sill_settings::Px| Px(f32::from(value.0));
    DockMetrics {
        tile: px(dock.tile_size_px),
        gap: px(dock.tile_gap_px),
        pad: px(dock.pill_padding_px),
        dot: px(dock.running_dot_diameter_px),
        dot_gap: px(dock.running_dot_gap_px),
        floor: match dock.floor {
            DockFloor::Off => DockFloorSetting::Off,
            DockFloor::On => DockFloorSetting::On,
        },
    }
}

/// The pill's corner: a squircle of nominal radius `dock.pill_radius_px` (quire's
/// `Corner::Squircle`; its extent is capped at half the pill's height).
pub fn pill_corner(dock: &DockSettings) -> Corner {
    Corner::Squircle(Px(f32::from(dock.pill_radius_px.0)))
}

/// The blur region's corner radius for the pill: the circle the squircle's shadows follow,
/// which touches it at 45 degrees (`Corner::css`'s radius), in whole pixels.
pub fn pill_blur_radius(dock: &DockSettings) -> i32 {
    let share = ds::icon::plate::shadow_radius_share();
    // A radius key is at most u16::MAX, so the rounded product fits an i32.
    (f64::from(dock.pill_radius_px.0) * share).round() as i32
}

/// `MagnifyParams` for `dock` on an output `output_width_px` wide.
pub fn magnify_params(dock: &DockSettings, output_width_px: i32) -> MagnifyParams {
    let px = |v: u16| Cpx::px(i32::from(v));
    MagnifyParams {
        tile: px(dock.tile_size_px.0),
        magnified: px(dock.magnified_size_px.0),
        radius: px(dock.influence_radius_px.0),
        gap: px(dock.tile_gap_px.0),
        pad: px(dock.pill_padding_px.0),
        edge_clamp: px(dock.edge_clamp_px.0),
        output_width: Cpx::px(output_width_px),
    }
}

#[cfg(test)]
mod tests {
    use super::{dock_metrics, magnify_params};
    use crate::magnify::MagnifyParams;
    use ds_shell::tokens::dock::DockMetrics;
    use sill_model::units::Cpx;
    use sill_settings::DockSettings;

    #[test]
    fn default_settings_are_the_design_geometry() {
        assert_eq!(
            magnify_params(&DockSettings::default(), 1920),
            MagnifyParams::design_defaults(Cpx::px(1920))
        );
    }

    #[test]
    fn the_pill_blurs_under_the_squircles_shadow_circle() {
        use super::{pill_blur_radius, pill_corner};
        let dock = DockSettings::default();
        assert_eq!(pill_corner(&dock).css(), "19.45px", "0.884 x 22");
        assert_eq!(pill_blur_radius(&dock), 19);
    }

    #[test]
    fn default_settings_are_quires_dock_metrics() {
        assert_eq!(
            dock_metrics(&DockSettings::default()),
            DockMetrics::default()
        );
    }
}
