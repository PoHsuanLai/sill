//! The power menu's layer surface (design/20 §1.8): Overlay over the whole output (the scrim is
//! part of the surface, so no catcher is needed), no exclusive zone, keyboard `Exclusive`, input
//! everywhere, blur behind the sheet only.

use shell_host::{
    Anchor, BlurSpec, ElementId, ExclusiveZone, KeyboardMode, LayerConfig, LayerExtent, LayerLevel,
    LayerSize, Margin, OutputInfo, OutputTarget, RegionSpec,
};

use crate::surfaces::naming::namespace;

/// The sheet's element id: the surface's blur region.
pub const PANEL_ID: &str = "power-menu-panel";

/// The blur region's corner under the sheet: quire's `.ds-sheet` rounds with a plain
/// `border-radius: var(--r-panel)`, a circle of 14 (not the launcher card's squircle, whose
/// shadow circle is 12), and the panel element is the sheet's whole box.
pub const PANEL_BLUR_RADIUS: i32 = 14;

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
        layer: LayerLevel::Overlay,
        anchor: Anchor::ALL,
        size: LayerSize {
            width: LayerExtent::Fill,
            height: LayerExtent::Fill,
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
    use shell_host::{
        Anchor, BlurSpec, ExclusiveZone, KeyboardMode, LayerExtent, LayerLevel, RegionSpec,
    };

    #[test]
    fn the_power_menu_is_a_keyboard_exclusive_full_output_overlay() {
        let layer = power_menu_layer(&output(Some("DP-1")));
        assert_eq!(layer.namespace, "sill-power-menu-DP-1");
        assert_eq!(layer.layer, LayerLevel::Overlay);
        assert_eq!(layer.anchor, Anchor::ALL);
        assert_eq!(
            (layer.size.width, layer.size.height),
            (LayerExtent::Fill, LayerExtent::Fill)
        );
        assert_eq!(layer.exclusive, ExclusiveZone::Ignore);
        assert_eq!(layer.keyboard, KeyboardMode::Exclusive);
        assert_eq!(layer.input, RegionSpec::Whole);
        assert_eq!(layer.blur, BlurSpec::Region(panel_region()));
    }
}
