//! The bar's layer surface: Top, across the top of the output, tall enough for the menu bar
//! above and the expanded bar (thumbnails and names) below it, reserving nothing (the menu bar's
//! own surface is spawned after it and draws over its top), no keyboard, no blur. Input goes to
//! the bar's own box only, so the rest of the surface lets the pointer through to the windows.
//!
//! The compositor keeps [`STRIP_PX`] free of windows while Mission Control is open, which is less
//! than this surface: the lower part of the expanded bar lies over the top of the windows, as on
//! the Mac (FINDINGS `mc2`).

use ds::style::tokens::shell_scale::SHELL_SCALE;
use shell_host::{LayerConfig, OutputInfo};
use sill_model::desktop::BAR_EXPANDED_PX;
use sill_settings::ShellFile;

use sill_ui_kit::surface::{LayerPreset, PanelBlur};

/// The strip casement reserves at the top of an output for this bar while Mission Control is
/// open (casement FINDINGS, `wave-2026-10-15/overview`): the windows spread below it.
pub const STRIP_PX: u32 = 112;

/// The `id` of the bar's element, which the input region follows.
pub(super) const BAR_ID: &str = "spaces-bar";

/// The surface's height: the menu bar's, then the expanded bar's.
pub(super) fn surface_px(menu_bar_px: u32) -> u32 {
    menu_bar_px + BAR_EXPANDED_PX
}

/// The bar on `output`.
pub fn layer(output: &OutputInfo, _settings: &ShellFile) -> LayerConfig {
    LayerPreset::Strip {
        height: surface_px(u32::from(SHELL_SCALE.bar.0)),
        bar: PanelBlur {
            id: BAR_ID,
            radius: 0,
        },
    }
    .config("spaces-bar", output)
}

#[cfg(test)]
mod tests {
    use super::{BAR_ID, STRIP_PX, layer as bar_layer, surface_px};
    use ds::style::tokens::shell_scale::SHELL_SCALE;
    use shell_host::{
        Anchor, BlurSpec, ElementId, ExclusiveZone, KeyboardMode, LayerExtent, LayerLevel,
        RegionSpec,
    };
    use sill_model::desktop::BAR_EXPANDED_PX;
    use sill_settings::ShellFile;
    use sill_ui_kit::harness::fake::output_named as output;

    #[test]
    fn the_bar_is_a_top_strip_that_reserves_nothing_takes_no_keys_and_takes_input_on_its_own_box() {
        let settings = ShellFile::default();
        let layer = bar_layer(&output(Some("DP-1")), &settings);
        assert_eq!(layer.namespace, "sill-spaces-bar-DP-1");
        assert_eq!(layer.layer, LayerLevel::Top);
        assert_eq!(layer.anchor, Anchor::TOP | Anchor::LEFT | Anchor::RIGHT);
        assert_eq!(layer.size.width, LayerExtent::Fill);
        assert_eq!(layer.exclusive, ExclusiveZone::Ignore);
        assert_eq!(layer.keyboard, KeyboardMode::None);
        assert_eq!(layer.blur, BlurSpec::None);
        assert_eq!(
            layer.input,
            RegionSpec::Element {
                id: ElementId::new(BAR_ID),
                radius: 0,
                inflate: 0
            }
        );
    }

    #[test]
    fn the_surface_holds_the_menu_bar_and_the_expanded_bar_and_is_taller_than_the_reserve() {
        let settings = ShellFile::default();
        let layer = bar_layer(&output(Some("DP-1")), &settings);
        let menu = u32::from(SHELL_SCALE.bar.0);
        assert_eq!(layer.size.height, LayerExtent::Px(menu + BAR_EXPANDED_PX));
        assert_eq!(surface_px(24), 24 + BAR_EXPANDED_PX);
        assert!(surface_px(menu) > STRIP_PX);
    }
}
