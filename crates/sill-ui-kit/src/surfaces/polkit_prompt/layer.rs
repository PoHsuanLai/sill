//! The polkit prompt's layer surface (design/20 §1.10): Overlay over the whole output (the
//! scrim is part of the surface), no exclusive zone, keyboard `Exclusive`, input everywhere,
//! blur behind the sheet only. The power menu's shape.

use shell_host::{
    Anchor, BlurSpec, ElementId, ExclusiveZone, KeyboardMode, LayerConfig, LayerExtent, LayerLevel,
    LayerSize, Margin, OutputInfo, OutputTarget, RegionSpec,
};

use sill_settings::ShellFile;

use crate::surfaces::naming::namespace;

/// The sheet's element id: the surface's blur region.
pub(super) const PANEL_ID: &str = "polkit-panel";

/// The sheet's corner: quire's `.ds-sheet` rounds with `--r-panel` (14), as the power menu's.
const PANEL_BLUR_RADIUS: i32 = 14;

/// The polkit prompt on `output`.
pub fn layer(output: &OutputInfo, _settings: &ShellFile) -> LayerConfig {
    LayerConfig {
        namespace: namespace("polkit", output),
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
    use super::layer as polkit_layer;
    use crate::surfaces::naming::tests::output;
    use shell_host::{Anchor, ExclusiveZone, KeyboardMode, LayerLevel, RegionSpec};
    use sill_settings::ShellFile;

    #[test]
    fn the_prompt_is_a_keyboard_exclusive_full_output_overlay() {
        let layer = polkit_layer(&output(Some("DP-1")), &ShellFile::default());
        assert_eq!(layer.namespace, "sill-polkit-DP-1");
        assert_eq!(layer.layer, LayerLevel::Overlay);
        assert_eq!(layer.anchor, Anchor::ALL);
        assert_eq!(layer.exclusive, ExclusiveZone::Ignore);
        assert_eq!(layer.keyboard, KeyboardMode::Exclusive);
        assert_eq!(layer.input, RegionSpec::Whole);
    }
}
