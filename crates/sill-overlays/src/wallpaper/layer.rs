//! The wallpaper's layer surface: Background, every edge, ignoring exclusive zones, no keyboard,
//! input over the whole output (a press of the secondary button opens the desktop's menu).

use shell_host::{LayerConfig, LayerLevel, OutputInfo};

use sill_ui_kit::surface::{LayerPreset, Pointer};

/// The wallpaper's layer configuration on `output`.
pub fn layer(output: &OutputInfo) -> LayerConfig {
    LayerPreset::Whole {
        level: LayerLevel::Background,
        pointer: Pointer::Takes,
    }
    .config("wallpaper", output)
}

#[cfg(test)]
mod tests {
    use super::layer;
    use shell_host::{Anchor, BlurSpec, ExclusiveZone, LayerLevel, RegionSpec};
    use sill_ui_kit::harness::fake::output_named as output;

    #[test]
    fn the_wallpaper_fills_the_background_and_takes_the_pointer_for_its_menu() {
        let config = layer(&output(Some("DP-1")));
        assert_eq!(config.namespace, "sill-wallpaper-DP-1");
        assert_eq!(config.layer, LayerLevel::Background);
        assert_eq!(config.anchor, Anchor::ALL);
        assert_eq!(config.exclusive, ExclusiveZone::Ignore);
        assert_eq!(config.input, RegionSpec::Whole);
        assert_eq!(config.blur, BlurSpec::None);
    }
}
