//! The notification center's layer surface: an Overlay over the output below the bar (all four
//! edges anchored, exclusive zone `Auto`, so the compositor keeps it clear of the bar's and the
//! dock's reserve), keyboard `OnDemand` (design/20 §1.6: Escape closes, arrows move), input
//! everywhere: the transparent part left of the panel is the click-outside catcher, as the power
//! menu's scrim is. The panel is a column at the right edge, `notifications.center_width_px`
//! wide, sliding in from the right; the blur is the panel's.
//!
//! Why a layer and not an xdg popup from the clock (design/20 §1.6 allows either): macOS 11 on
//! opens Notification Center from the clock as a full-height column pinned to the right edge,
//! not a menu hanging from the item; the history can fill the output's height, and the widgets
//! slot below it wants that height too; `sill notifications toggle` must open it without a
//! pointer serial, which a grabbing popup needs (the control center's F179 problem); and a popup
//! would join the bar's hover switch, which Notification Center does not.

use shell_host::{
    Anchor, BlurSpec, ElementId, ExclusiveZone, Extent, KeyboardMode, Layer, LayerConfig,
    LayerSize, Margin, OutputInfo, OutputTarget, RegionSpec,
};

use crate::surfaces::naming::namespace;

/// The element id of the column inside quire's `Panel` (which takes no id of its own): the blur
/// region, grown by the panel's padding to the panel's own box.
pub const PANEL_ID: &str = "notification-center-panel";

/// The panel's corner: quire's Popover (`--r-panel`, 14).
pub const PANEL_RADIUS: i32 = 14;

/// quire's `Panel` pads its content by `--s-12`: the column inside is that much smaller.
pub const PANEL_PADDING: i32 = 12;

/// The panel's blur region.
pub fn panel_region() -> RegionSpec {
    RegionSpec::Element {
        id: ElementId::new(PANEL_ID),
        radius: PANEL_RADIUS,
        inflate: PANEL_PADDING,
    }
}

/// The notification center on `output`.
pub fn center_layer(output: &OutputInfo) -> LayerConfig {
    LayerConfig {
        namespace: namespace("notification-center", output),
        layer: Layer::Overlay,
        anchor: Anchor::ALL,
        size: LayerSize {
            width: Extent::Fill,
            height: Extent::Fill,
        },
        exclusive: ExclusiveZone::Auto,
        margin: Margin::ZERO,
        keyboard: KeyboardMode::OnDemand,
        output: OutputTarget::Output(output.id),
        blur: BlurSpec::Region(panel_region()),
        input: RegionSpec::Whole,
    }
}

#[cfg(test)]
mod tests {
    use super::{center_layer, panel_region};
    use crate::surfaces::naming::tests::output;
    use shell_host::{Anchor, BlurSpec, ExclusiveZone, Extent, KeyboardMode, Layer, RegionSpec};

    #[test]
    fn the_center_covers_the_output_below_the_bar_and_takes_the_keyboard_on_demand() {
        let config = center_layer(&output(Some("DP-1")));
        assert_eq!(config.namespace, "sill-notification-center-DP-1");
        assert_eq!(config.layer, Layer::Overlay);
        assert_eq!(config.anchor, Anchor::ALL);
        assert_eq!(config.size.width, Extent::Fill);
        assert_eq!(config.size.height, Extent::Fill);
        assert_eq!(config.exclusive, ExclusiveZone::Auto);
        assert_eq!(config.keyboard, KeyboardMode::OnDemand);
        assert_eq!(config.input, RegionSpec::Whole);
        assert_eq!(config.blur, BlurSpec::Region(panel_region()));
    }
}
