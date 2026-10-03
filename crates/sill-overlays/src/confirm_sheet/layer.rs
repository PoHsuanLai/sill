//! The confirmation sheet's layer surface: Overlay over the whole output, no exclusive zone,
//! keyboard `Exclusive` (a question that asks must take the keys, Escape denies), input
//! everywhere, blur behind the sheet only. The polkit prompt's shape.

use shell_host::{
    Anchor, BlurSpec, ElementId, ExclusiveZone, KeyboardMode, LayerConfig, LayerExtent, LayerLevel,
    LayerSize, Margin, OutputInfo, OutputTarget, RegionSpec,
};

use sill_settings::ShellFile;

use sill_ui_kit::surface::namespace;

/// The sheet's element id: the surface's blur region.
pub(super) const PANEL_ID: &str = "confirm-panel";

/// The sheet's corner: quire's `.ds-sheet` rounds with `--r-panel` (14).
const PANEL_BLUR_RADIUS: i32 = 14;

/// The confirmation sheet on `output`.
pub fn layer(output: &OutputInfo, _settings: &ShellFile) -> LayerConfig {
    LayerConfig {
        namespace: namespace("confirm", output),
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
        blur: BlurSpec::Region(RegionSpec::Element {
            id: ElementId::new(PANEL_ID),
            radius: PANEL_BLUR_RADIUS,
            inflate: 0,
        }),
        input: RegionSpec::Whole,
    }
}

#[cfg(test)]
mod tests {
    use super::layer as confirm_layer;
    use shell_host::{Anchor, ExclusiveZone, KeyboardMode, LayerLevel, RegionSpec};
    use sill_settings::ShellFile;
    use sill_ui_kit::harness::fake::output_named as output;

    #[test]
    fn the_sheet_is_a_keyboard_exclusive_full_output_overlay() {
        let layer = confirm_layer(&output(Some("DP-1")), &ShellFile::default());
        assert_eq!(layer.namespace, "sill-confirm-DP-1");
        assert_eq!(layer.layer, LayerLevel::Overlay);
        assert_eq!(layer.anchor, Anchor::ALL);
        assert_eq!(layer.exclusive, ExclusiveZone::Ignore);
        assert_eq!(layer.keyboard, KeyboardMode::Exclusive);
        assert_eq!(layer.input, RegionSpec::Whole);
    }
}
