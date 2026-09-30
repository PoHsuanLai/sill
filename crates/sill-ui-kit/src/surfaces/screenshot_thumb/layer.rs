//! The thumbnail's surface (design/20 §1.13): Overlay, anchored `BOTTOM | RIGHT`, exclusive
//! zone `Auto` (above the dock's reserve, as macOS's thumbnail sits above the Dock), no
//! keyboard, input and blur the thumbnail's own card (`Element("thumb")`), with room around
//! the card for the Toast material's shadow and the swipe's travel.

use shell_host::{
    Anchor, BlurSpec, ElementId, ExclusiveZone, KeyboardMode, LayerConfig, LayerExtent, LayerLevel,
    LayerSize, Margin, OutputInfo, OutputTarget, RegionSpec,
};

use sill_settings::ShellFile;

use crate::surface::namespace;

/// The card's element id: its input and blur region.
pub const THUMB_ID: &str = "thumb";

/// The card's corner (the Toast material's, as a banner's: 16).
pub const THUMB_RADIUS: i32 = 16;

/// The card's width in logical px, quire's `ShotThumbnail` default. design/20 names none;
/// macOS's is about a sixth of a 1440 px screen.
pub const THUMB_WIDTH: u32 = 240;

/// quire's mat round the picture.
const THUMB_MAT: u32 = 4;

/// The card at its tallest: a 16:10 picture box inside the mat (quire holds the box between 2:1
/// and 16:10, so no picture makes it taller).
pub const THUMB_HEIGHT: u32 = (THUMB_WIDTH - 2 * THUMB_MAT) * 10 / 16 + 2 * THUMB_MAT;

/// Transparent room around the card: the shadow (the banners' 40) and, to the right, the swipe.
pub const ROOM: u32 = 40;

/// The card's region.
pub fn thumb_region() -> RegionSpec {
    RegionSpec::Element {
        id: ElementId::new(THUMB_ID),
        radius: THUMB_RADIUS,
        inflate: 0,
    }
}

/// The thumbnail on `output`: a column `THUMB_WIDTH` plus room wide, as tall as a 16:10 picture
/// plus room (a taller picture is letterboxed in the card).
pub fn layer(output: &OutputInfo, _settings: &ShellFile) -> LayerConfig {
    LayerConfig {
        namespace: namespace("screenshot-thumb", output),
        layer: LayerLevel::Overlay,
        anchor: Anchor::BOTTOM | Anchor::RIGHT,
        size: LayerSize {
            width: LayerExtent::Px(THUMB_WIDTH + 2 * ROOM),
            height: LayerExtent::Px(THUMB_HEIGHT + 2 * ROOM),
        },
        exclusive: ExclusiveZone::Auto,
        margin: Margin::ZERO,
        keyboard: KeyboardMode::None,
        output: OutputTarget::Output(output.id),
        blur: BlurSpec::Region(thumb_region()),
        input: thumb_region(),
    }
}

#[cfg(test)]
mod tests {
    use sill_settings::ShellFile;

    use super::{ROOM, THUMB_WIDTH, layer, thumb_region};
    use crate::harness::fake::output_named as output;
    use shell_host::{Anchor, BlurSpec, ExclusiveZone, KeyboardMode, LayerExtent, LayerLevel};

    #[test]
    fn the_thumbnail_sits_at_the_bottom_right_and_takes_its_card_only() {
        let config = layer(&output(Some("DP-1")), &ShellFile::default());
        assert_eq!(config.namespace, "sill-screenshot-thumb-DP-1");
        assert_eq!(config.layer, LayerLevel::Overlay);
        assert_eq!(config.anchor, Anchor::BOTTOM | Anchor::RIGHT);
        assert_eq!(config.size.width, LayerExtent::Px(THUMB_WIDTH + 2 * ROOM));
        assert_eq!(config.size.height, LayerExtent::Px(153 + 2 * ROOM));
        assert_eq!(config.exclusive, ExclusiveZone::Auto);
        assert_eq!(config.keyboard, KeyboardMode::None);
        assert_eq!(config.input, thumb_region());
        assert_eq!(config.blur, BlurSpec::Region(thumb_region()));
    }
}
