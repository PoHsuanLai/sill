//! The dock's layer surface (design/10 §10.3.1, §10.7): Top, bottom edge, tall enough for a
//! magnified tile plus a bounce (`gut + pad + S + T`), reserving `gut + H_p` unless auto-hide
//! is on, no keyboard (OnDemand while its menu is open, shell-host's rule), input and blur
//! following the `dock-hit` and `dock-pill` elements.
//!
//! Anchored bottom, left and right with a full-width buffer: the plan's `BOTTOM` alone would
//! need a width that follows magnification every frame; a full-width transparent surface with
//! an element input region does not.

use shell_host::{
    Anchor, BlurSpec, ExclusiveZone, KeyboardMode, LayerConfig, LayerExtent, LayerLevel, LayerSize,
    Margin, OutputInfo, OutputTarget,
};
use sill_settings::{AutoHide, DockSettings};

use crate::surfaces::dock::machine::Reveal;
use crate::surfaces::naming::namespace;

/// design/10 §10.3.1: the gutter under the pill, the card inset (settled). The pill's own
/// padding is `dock.pill_padding_px`.
pub const GUTTER_PX: u32 = 8;

/// The dock's layer configuration on `output`.
pub fn dock_layer(output: &OutputInfo, dock: &DockSettings) -> LayerConfig {
    let tile = u32::from(dock.tile_size_px.0);
    let magnified = u32::from(dock.magnified_size_px.0);
    let pad = u32::from(dock.pill_padding_px.0);
    let reveal = match dock.autohide {
        AutoHide::Off => Reveal::Fixed,
        AutoHide::On => Reveal::Hidden,
    };
    let reserve = GUTTER_PX + tile + 2 * pad;
    LayerConfig {
        namespace: namespace("dock", output),
        layer: LayerLevel::Top,
        anchor: Anchor::BOTTOM | Anchor::LEFT | Anchor::RIGHT,
        size: LayerSize {
            width: LayerExtent::Fill,
            height: LayerExtent::Px(GUTTER_PX + pad + magnified + tile),
        },
        exclusive: match reveal {
            Reveal::Fixed => ExclusiveZone::Reserve(reserve),
            _ => ExclusiveZone::Reserve(0),
        },
        margin: Margin::ZERO,
        keyboard: KeyboardMode::None,
        output: OutputTarget::Output(output.id),
        blur: match reveal {
            Reveal::Fixed => BlurSpec::Region(crate::surfaces::dock::machine::pill_region(
                crate::surfaces::dock::geometry::pill_blur_radius(dock),
            )),
            _ => BlurSpec::None,
        },
        input: match reveal {
            Reveal::Fixed => crate::surfaces::dock::machine::hit_region(),
            _ => crate::surfaces::dock::machine::strip_region(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::dock_layer;
    use crate::surfaces::naming::tests::output;
    use shell_host::{Anchor, BlurSpec, ExclusiveZone, KeyboardMode, LayerExtent, LayerLevel};
    use sill_settings::{AutoHide, DockSettings};

    #[test]
    fn the_dock_is_158_tall_and_reserves_68_unless_it_hides() {
        // gut 8 + pad 6 + magnified 96 + tile 48; gut 8 + tile 48 + 2 x pad 6.
        let config = dock_layer(&output(Some("DP-1")), &DockSettings::default());
        assert_eq!(config.namespace, "sill-dock-DP-1");
        assert_eq!(config.layer, LayerLevel::Top);
        assert_eq!(config.anchor, Anchor::BOTTOM | Anchor::LEFT | Anchor::RIGHT);
        assert_eq!(config.size.height, LayerExtent::Px(158));
        assert_eq!(config.exclusive, ExclusiveZone::Reserve(68));
        assert_eq!(config.keyboard, KeyboardMode::None);
        assert!(matches!(config.blur, BlurSpec::Region(_)));
        let hiding = DockSettings {
            autohide: AutoHide::On,
            ..DockSettings::default()
        };
        let config = dock_layer(&output(Some("DP-1")), &hiding);
        assert_eq!(config.exclusive, ExclusiveZone::Reserve(0));
        assert_eq!(config.blur, BlurSpec::None);
    }
}
