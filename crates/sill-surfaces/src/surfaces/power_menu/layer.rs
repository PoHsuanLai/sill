//! The power menu's layer surface (design/20 §1.8): Overlay over the whole output (the scrim is
//! part of the surface, so no catcher is needed), no exclusive zone, keyboard `Exclusive`, input
//! everywhere, blur behind the sheet only.

use shell_host::{
    Anchor, BlurSpec, ElementId, ExclusiveZone, Extent, KeyboardMode, Layer, LayerConfig,
    LayerSize, Margin, OutputInfo, OutputTarget, RegionSpec,
};

use crate::surfaces::naming::namespace;

/// The sheet's element id: the surface's blur region.
pub const PANEL_ID: &str = "power-menu-panel";

/// The blur region's corner under the sheet: the circle quire's `--r-panel` squircle (14)
/// follows, as the launcher card's (`CARD_BLUR_RADIUS`). Agent (d) checks it against the sheet
/// quire draws.
pub const PANEL_BLUR_RADIUS: i32 = 12;

/// The sheet's blur region.
pub fn panel_region() -> RegionSpec {
    RegionSpec::Element {
        id: ElementId::new(PANEL_ID),
        radius: PANEL_BLUR_RADIUS,
        inflate: 0,
    }
}

/// The power menu on `output`.
pub fn power_menu_layer(output: &OutputInfo) -> LayerConfig {
    LayerConfig {
        namespace: namespace("power-menu", output),
        layer: Layer::Overlay,
        anchor: Anchor::ALL,
        size: LayerSize {
            width: Extent::Fill,
            height: Extent::Fill,
        },
        exclusive: ExclusiveZone::Ignore,
        margin: Margin::ZERO,
        keyboard: KeyboardMode::Exclusive,
        output: OutputTarget::Output(output.id),
        blur: BlurSpec::Region(panel_region()),
        input: RegionSpec::Whole,
    }
}

#[cfg(test)]
mod tests {
    use super::{panel_region, power_menu_layer};
    use crate::surfaces::naming::tests::output;
    use shell_host::{Anchor, BlurSpec, ExclusiveZone, Extent, KeyboardMode, Layer, RegionSpec};

    #[test]
    fn the_power_menu_is_a_keyboard_exclusive_full_output_overlay() {
        let layer = power_menu_layer(&output(Some("DP-1")));
        assert_eq!(layer.namespace, "sill-power-menu-DP-1");
        assert_eq!(layer.layer, Layer::Overlay);
        assert_eq!(layer.anchor, Anchor::ALL);
        assert_eq!(
            (layer.size.width, layer.size.height),
            (Extent::Fill, Extent::Fill)
        );
        assert_eq!(layer.exclusive, ExclusiveZone::Ignore);
        assert_eq!(layer.keyboard, KeyboardMode::Exclusive);
        assert_eq!(layer.input, RegionSpec::Whole);
        assert_eq!(layer.blur, BlurSpec::Region(panel_region()));
    }
}
