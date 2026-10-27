//! The dock's settings and tokens as `crate::magnify`'s parameters and as quire's `DockMetrics`
//! (the macOS polish pass: tile 48, gap 8, pad 6, dot 4 centred 3 under the tile, floor off).

use crate::dock::tokens::DockTokens;
use crate::magnify::MagnifyParams;
use ds::prelude::*;
use ds::style::tokens::shape::Corner;
use sill_model::units::Cpx;
use sill_settings::{DockFloor, DockSettings};
use sill_shell_kit::tokens::dock::{DockFloorSetting, DockMetrics};

/// quire's dock tokens from `dock.tile_size_px` and `dock.floor`: written inline around the
/// dock's root (`DockMetrics::style_attr`), where `DockFloor` reads them. The gap, padding and
/// dot are stylesheet tokens (`DockToken`), not written here.
pub fn dock_metrics(dock: &DockSettings) -> DockMetrics {
    DockMetrics {
        tile: Px(f32::from(dock.tile_size_px.0)),
        floor: match dock.floor {
            DockFloor::Off => DockFloorSetting::Off,
            DockFloor::On => DockFloorSetting::On,
        },
    }
}

/// The pill's corner: a squircle of nominal radius `--sill-dock-pill-radius` (quire's
/// `Corner::Squircle`; its extent is capped at half the pill's height).
pub fn pill_corner(tokens: &DockTokens) -> Corner {
    Corner::Squircle(Px(f32::from(tokens.pill_radius)))
}

/// The blur region's corner radius for the pill: the circle the squircle's shadows follow,
/// which touches it at 45 degrees (`Corner::css`'s radius), in whole pixels.
pub fn pill_blur_radius(tokens: &DockTokens) -> i32 {
    let share = ds::style::tokens::plate::shadow_radius_share();
    // A radius is at most u16::MAX, so the rounded product fits an i32.
    (f64::from(tokens.pill_radius) * share).round() as i32
}

/// How far from the pointer icons still grow.
const INFLUENCE_RADIUS_PX: i32 = 72;

/// `MagnifyParams` for `dock` on an output `output_width_px` wide.
pub fn magnify_params(
    dock: &DockSettings,
    tokens: &DockTokens,
    output_width_px: i32,
) -> MagnifyParams {
    let px = |v: u16| Cpx::px(i32::from(v));
    MagnifyParams {
        tile: px(dock.tile_size_px.0),
        magnified: px(dock.magnified_size_px.0),
        radius: Cpx::px(INFLUENCE_RADIUS_PX),
        gap: px(tokens.gap),
        pad: px(tokens.pad),
        edge_clamp: px(tokens.edge_clamp),
        output_width: Cpx::px(output_width_px),
    }
}

#[cfg(test)]
mod tests {
    use super::{dock_metrics, magnify_params};
    use crate::dock::tokens::DockTokens;
    use crate::magnify::MagnifyParams;
    use sill_model::units::Cpx;
    use sill_settings::DockSettings;
    use sill_shell_kit::tokens::dock::DockMetrics;

    #[test]
    fn default_settings_are_the_design_geometry() {
        assert_eq!(
            magnify_params(&DockSettings::default(), &DockTokens::default(), 1920),
            MagnifyParams::design_defaults(Cpx::px(1920))
        );
    }

    #[test]
    fn the_pill_blurs_under_the_squircles_shadow_circle() {
        use super::{pill_blur_radius, pill_corner};
        let tokens = DockTokens::default();
        assert_eq!(pill_corner(&tokens).css(), "19.45px", "0.884 x 22");
        assert_eq!(pill_blur_radius(&tokens), 19);
    }

    #[test]
    fn default_settings_are_quires_dock_metrics() {
        assert_eq!(
            dock_metrics(&DockSettings::default()),
            DockMetrics::default()
        );
    }
}
