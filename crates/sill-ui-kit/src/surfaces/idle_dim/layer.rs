//! The dim's layer surface: Overlay, the whole output (anchored to all four edges, exclusive zone
//! `Ignore`, so it covers the bar's and the dock's reserve too), no keyboard, no blur, and an
//! empty input region, so every pointer event passes to what is under it and the compositor's
//! idle tracking sees the input as it would without the scrim. The session-lock surfaces are
//! above every layer surface, so a lock taken while dimmed covers it.

use shell_host::{
    Anchor, BlurSpec, ExclusiveZone, KeyboardMode, LayerConfig, LayerExtent, LayerLevel, LayerSize,
    Margin, OutputInfo, OutputTarget, RegionSpec,
};

use crate::surfaces::naming::namespace;

/// The dim's layer configuration on `output`.
pub fn layer(output: &OutputInfo) -> LayerConfig {
    LayerConfig {
        namespace: namespace("idle-dim", output),
        layer: LayerLevel::Overlay,
        anchor: Anchor::TOP | Anchor::BOTTOM | Anchor::LEFT | Anchor::RIGHT,
        size: LayerSize {
            width: LayerExtent::Fill,
            height: LayerExtent::Fill,
        },
        exclusive: ExclusiveZone::Ignore,
        margin: Margin::ZERO,
        keyboard: KeyboardMode::None,
        output: OutputTarget::Output(output.id),
        blur: BlurSpec::None,
        input: RegionSpec::Empty,
    }
}

#[cfg(test)]
mod tests {
    use shell_host::{
        Anchor, BlurSpec, ExclusiveZone, KeyboardMode, LayerExtent, LayerLevel, OutputTarget,
        RegionSpec,
    };

    use super::layer;
    use crate::surfaces::naming::tests::output;

    #[test]
    fn the_dim_covers_the_output_and_takes_nothing() {
        let info = output(Some("DP-1"));
        let config = layer(&info);
        assert_eq!(config.namespace, "sill-idle-dim-DP-1");
        assert_eq!(config.layer, LayerLevel::Overlay);
        assert_eq!(
            config.anchor,
            Anchor::TOP | Anchor::BOTTOM | Anchor::LEFT | Anchor::RIGHT
        );
        assert_eq!(config.size.width, LayerExtent::Fill);
        assert_eq!(config.size.height, LayerExtent::Fill);
        assert_eq!(config.exclusive, ExclusiveZone::Ignore);
        assert_eq!(config.keyboard, KeyboardMode::None);
        assert_eq!(config.blur, BlurSpec::None);
        assert_eq!(config.input, RegionSpec::Empty);
        assert_eq!(config.output, OutputTarget::Output(info.id));
    }
}
