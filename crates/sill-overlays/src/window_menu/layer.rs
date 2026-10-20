//! The window menu surface's layer: Overlay over the whole output, no keyboard, no blur and an
//! empty input region, so it takes nothing itself and every pointer event passes to what is
//! under it. It exists to be the parent of the menu's popup, which the compositor places in the
//! layer's coordinates.

use shell_host::{LayerConfig, LayerLevel, OutputInfo};

use sill_ui_kit::surface::{LayerPreset, Pointer};

/// The window menu surface's layer configuration on `output`.
pub fn layer(output: &OutputInfo) -> LayerConfig {
    LayerPreset::Whole {
        level: LayerLevel::Overlay,
        pointer: Pointer::PassesThrough,
    }
    .config("window-menu", output)
}

#[cfg(test)]
mod tests {
    use shell_host::{
        Anchor, BlurSpec, ExclusiveZone, KeyboardMode, LayerExtent, LayerLevel, OutputTarget,
        RegionSpec,
    };

    use super::layer;
    use sill_ui_kit::harness::fake::output_named as output;

    #[test]
    fn the_surface_covers_the_output_and_takes_nothing() {
        let info = output(Some("DP-1"));
        let config = layer(&info);
        assert_eq!(config.namespace, "sill-window-menu-DP-1");
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
