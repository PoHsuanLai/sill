//! The switcher's layer surface (design/20 §1.11): Overlay over the whole output with the panel
//! centred in it, no exclusive zone, keyboard `Exclusive` (it must see the modifier's release),
//! input and blur on the panel only, so a click beside it reaches the window below.

use shell_host::{
    Anchor, BlurSpec, ElementId, ExclusiveZone, Extent, KeyboardMode, Layer, LayerConfig,
    LayerSize, Margin, OutputInfo, OutputTarget, RegionSpec,
};

use crate::surfaces::naming::namespace;

/// The panel's element id: the surface's input and blur region.
pub const PANEL_ID: &str = "switcher-panel";

/// The panel's corner: `Material::Osd`'s radius (design/13 §13.3.5: 18).
pub const PANEL_RADIUS: i32 = 18;

fn panel_region() -> RegionSpec {
    RegionSpec::Element {
        id: ElementId::new(PANEL_ID),
        radius: PANEL_RADIUS,
        inflate: 0,
    }
}

/// The switcher on `output`.
pub fn switcher_layer(output: &OutputInfo) -> LayerConfig {
    LayerConfig {
        namespace: namespace("switcher", output),
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
        input: panel_region(),
    }
}

#[cfg(test)]
mod tests {
    use super::{panel_region, switcher_layer};
    use crate::surfaces::naming::tests::output;
    use shell_host::{Anchor, BlurSpec, ExclusiveZone, KeyboardMode, Layer};

    #[test]
    fn the_switcher_is_a_keyboard_exclusive_overlay_taking_input_on_its_panel() {
        let layer = switcher_layer(&output(Some("DP-1")));
        assert_eq!(layer.namespace, "sill-switcher-DP-1");
        assert_eq!(layer.layer, Layer::Overlay);
        assert_eq!(layer.anchor, Anchor::ALL);
        assert_eq!(layer.exclusive, ExclusiveZone::Ignore);
        assert_eq!(layer.keyboard, KeyboardMode::Exclusive);
        assert_eq!(layer.input, panel_region());
        assert_eq!(layer.blur, BlurSpec::Region(panel_region()));
    }
}
