//! Edit Widgets' layer surface: an Overlay over the output (all four edges anchored, exclusive
//! zone `Auto`, so its box is the desktop widget layer's and the grid it measures is the one
//! the desktop's widgets sit on), keyboard `OnDemand` (Escape closes), input everywhere: the
//! part outside the panel is the click-outside catcher. The blur is the panel's.

use shell_host::{
    Anchor, BlurSpec, ElementId, ExclusiveZone, KeyboardMode, LayerConfig, LayerExtent, LayerLevel,
    LayerSize, Margin, OutputInfo, OutputTarget, RegionSpec,
};

use crate::panel::{PANEL_PADDING, PANEL_RADIUS};
use crate::surface::namespace;

/// The element id of the gallery's column inside quire's `Panel`: the blur region, grown by the
/// panel's padding to the panel's own box.
pub const EDIT_PANEL_ID: &str = "edit-widgets-panel";

/// Edit Widgets on `output`.
pub(super) fn layer(output: &OutputInfo) -> LayerConfig {
    LayerConfig {
        namespace: namespace("edit-widgets", output),
        layer: LayerLevel::Overlay,
        anchor: Anchor::ALL,
        size: LayerSize {
            width: LayerExtent::Fill,
            height: LayerExtent::Fill,
        },
        exclusive: ExclusiveZone::Auto,
        margin: Margin::ZERO,
        keyboard: KeyboardMode::OnDemand,
        output: OutputTarget::Output(output.id),
        blur: BlurSpec::Region(RegionSpec::Element {
            id: ElementId::new(EDIT_PANEL_ID),
            radius: PANEL_RADIUS,
            inflate: PANEL_PADDING,
        }),
        input: RegionSpec::Whole,
    }
}

#[cfg(test)]
mod tests {
    use super::layer;
    use crate::harness::fake::output_named as output;
    use shell_host::{Anchor, ExclusiveZone, KeyboardMode, LayerLevel, RegionSpec};

    #[test]
    fn the_sheet_covers_the_output_below_the_bar_and_takes_the_keyboard_on_demand() {
        let config = layer(&output(Some("DP-1")));
        assert_eq!(config.namespace, "sill-edit-widgets-DP-1");
        assert_eq!(config.layer, LayerLevel::Overlay);
        assert_eq!(config.anchor, Anchor::ALL);
        assert_eq!(config.exclusive, ExclusiveZone::Auto);
        assert_eq!(config.keyboard, KeyboardMode::OnDemand);
        assert_eq!(config.input, RegionSpec::Whole);
    }
}
