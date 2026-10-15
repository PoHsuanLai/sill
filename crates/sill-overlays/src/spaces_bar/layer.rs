//! The bar's layer surface: Top, across the top of the output at the height the compositor keeps
//! free of windows during Mission Control, reserving nothing (the menu bar's own surface is
//! spawned after it and draws over its top), no keyboard, no blur, input everywhere on it.

use shell_host::{
    Anchor, BlurSpec, ExclusiveZone, KeyboardMode, LayerConfig, LayerExtent, LayerLevel, LayerSize,
    Margin, OutputInfo, OutputTarget, RegionSpec,
};
use sill_settings::ShellFile;

use sill_ui_kit::surface::namespace;

/// The strip casement reserves at the top of an output for this bar while Mission Control is
/// open (casement FINDINGS, `wave-2026-10-15/overview`): the windows spread below it.
pub const STRIP_PX: u32 = 112;

/// The bar on `output`.
pub fn layer(output: &OutputInfo, _settings: &ShellFile) -> LayerConfig {
    LayerConfig {
        namespace: namespace("spaces-bar", output),
        layer: LayerLevel::Top,
        anchor: Anchor::TOP | Anchor::LEFT | Anchor::RIGHT,
        size: LayerSize {
            width: LayerExtent::Fill,
            height: LayerExtent::Px(STRIP_PX),
        },
        exclusive: ExclusiveZone::Ignore,
        margin: Margin::ZERO,
        keyboard: KeyboardMode::None,
        output: OutputTarget::Output(output.id),
        blur: BlurSpec::None,
        input: RegionSpec::Whole,
    }
}

#[cfg(test)]
mod tests {
    use super::{STRIP_PX, layer as bar_layer};
    use shell_host::{
        Anchor, BlurSpec, ExclusiveZone, KeyboardMode, LayerExtent, LayerLevel, RegionSpec,
    };
    use sill_settings::ShellFile;
    use sill_ui_kit::harness::fake::output_named as output;

    #[test]
    fn the_bar_is_a_top_strip_across_the_output_that_reserves_nothing_and_takes_no_keys() {
        let layer = bar_layer(&output(Some("DP-1")), &ShellFile::default());
        assert_eq!(layer.namespace, "sill-spaces-bar-DP-1");
        assert_eq!(layer.layer, LayerLevel::Top);
        assert_eq!(layer.anchor, Anchor::TOP | Anchor::LEFT | Anchor::RIGHT);
        assert_eq!(layer.size.width, LayerExtent::Fill);
        assert_eq!(layer.size.height, LayerExtent::Px(STRIP_PX));
        assert_eq!(layer.exclusive, ExclusiveZone::Ignore);
        assert_eq!(layer.keyboard, KeyboardMode::None);
        assert_eq!(layer.input, RegionSpec::Whole);
        assert_eq!(layer.blur, BlurSpec::None);
    }
}
