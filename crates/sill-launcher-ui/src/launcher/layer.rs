//! The launcher's two layer surfaces and the panel's vertical position (design/13 §13.5:
//! `top = max(bar_h + 24, round(0.4 H - 230))`, every number a `launcher.*` key).

use shell_host::{
    Anchor, BlurSpec, ExclusiveZone, KeyboardMode, LayerConfig, LayerExtent, LayerLevel, LayerSize,
    Margin, OutputInfo, OutputTarget, Rect,
};
use sill_settings::{BarSettings, LauncherSettings, ShellFile};

use crate::launcher::hit::{card_region, panel_input};
use crate::launcher::preview::PREVIEW_WIDTH_PX;
use sill_ui_kit::links::PanelHit;
use sill_ui_kit::surface::namespace;

/// The panel's fixed size (design/13 §13.3.9, settled by the plan): the area the card is laid
/// out in, at most this tall.
pub const PANEL_WIDTH: u32 = 680;
pub const PANEL_HEIGHT: u32 = 460;

/// The room the panel may take: the card and, while the preview pane is shown, the pane beside
/// it (`CommandPalette { aside }` widens the card by `PREVIEW_WIDTH_PX`). The surface
/// is this wide for good; the card sits centred in it, so it widens by half the pane on each
/// side and its middle never moves.
pub const PANEL_ROOM_WIDTH: u32 = PANEL_WIDTH + PREVIEW_WIDTH_PX;

/// How far in from the room's left edge the card starts while the pane is hidden.
pub const CARD_INSET: u32 = (PANEL_ROOM_WIDTH - PANEL_WIDTH) / 2;

/// The surface's transparent room around the panel for the card's ambient shadow (the Sheet's
/// `0 24px 60px -18px`, material stack v2): it reaches 60 - 18 = 42 px beside the card and
/// 24 + 42 = 66 px below it, and a surface clips at its edges. The panel sits at the top of the
/// surface, centred, so the card's place on the output does not move.
pub const SHADOW_SIDE: u32 = 42;
pub const SHADOW_BELOW: u32 = 66;

/// Where the panel sits, as thousandths of the output's height (before the height bias).
const TOP_VERTICAL_FRACTION: i64 = 400;

/// The panel's top edge on an output `output_height` tall.
pub fn panel_top(output_height: i32, bar: &BarSettings, launcher: &LauncherSettings) -> i32 {
    let below_bar = i32::from(bar.height_px.0) + i32::from(launcher.top_bar_gap_px.0);
    // round(fraction/1000 x H) in integers: add half the divisor before dividing.
    let scaled = (TOP_VERTICAL_FRACTION * i64::from(output_height) + 500).div_euclid(1000);
    let centred =
        i32::try_from(scaled).unwrap_or(i32::MAX) - i32::from(launcher.top_centre_bias_px.0);
    below_bar.max(centred)
}

/// The palette card's element id: the panel's blur and input region.
pub const CARD_ID: &str = "launcher-card";

/// The card's corner: a squircle of quire's panel radius (`--r-panel`, 14; the macOS polish
/// pass), handed to the palette as `CommandPalette { corner }`.
pub const CARD_RADIUS: f32 = 14.0;

/// The blur and input regions' corner under the card: the circle its squircle's shadows follow
/// (`Corner::Squircle(14).css()`, 12.38 px), in whole pixels.
pub const CARD_BLUR_RADIUS: i32 = 12;

/// The actions menu's element id: set on quire's menu through `Common`, so the panel's blur
/// can cover the menu while it hangs below the card.
pub const MENU_ID: &str = "launcher-actions-menu";

/// The blur region's corner under the menu: quire's menu radius (`--r-menu`, 14) as the circle
/// its squircle's shadows follow, in whole pixels (as [`CARD_BLUR_RADIUS`]).
pub const MENU_BLUR_RADIUS: i32 = 12;

/// The panel: Overlay, anchored top only (centred horizontally), keyboard-exclusive, blurred
/// behind the palette card ([`CARD_ID`]). Its input starts as the whole panel; the panel narrows
/// it to the card once it has measured one (`hit::PanelHit`).
pub fn layer(output: &OutputInfo, settings: &ShellFile) -> LayerConfig {
    LayerConfig {
        namespace: namespace("launcher", output),
        layer: LayerLevel::Overlay,
        anchor: Anchor::TOP,
        size: LayerSize {
            width: LayerExtent::Px(PANEL_ROOM_WIDTH + 2 * SHADOW_SIDE),
            height: LayerExtent::Px(PANEL_HEIGHT + SHADOW_BELOW),
        },
        exclusive: ExclusiveZone::Ignore,
        margin: Margin {
            top: panel_top(output.logical.height, &settings.bar, &settings.launcher),
            ..Margin::ZERO
        },
        keyboard: KeyboardMode::Exclusive,
        output: OutputTarget::Output(output.id),
        blur: BlurSpec::Region(card_region()),
        input: panel_input(PanelHit::Whole),
    }
}

/// The panel's rectangle in its output's coordinates (its room, the card and the pane's; the
/// surface is wider and taller by the shadow's room): centred, [`panel_top`] down.
pub fn panel_rect(output: &OutputInfo, bar: &BarSettings, launcher: &LauncherSettings) -> Rect {
    let width = i32::try_from(PANEL_ROOM_WIDTH).unwrap_or(i32::MAX);
    Rect {
        x: (output.logical.width - width).div_euclid(2),
        y: panel_top(output.logical.height, bar, launcher),
        width,
        height: i32::try_from(PANEL_HEIGHT).unwrap_or(i32::MAX),
    }
}

#[cfg(test)]
mod tests {
    use super::{layer, panel_top};
    use shell_host::{Anchor, BlurSpec, KeyboardMode, LayerExtent, LayerLevel, RegionSpec};
    use sill_settings::{BarSettings, LauncherSettings, ShellFile};
    use sill_ui_kit::harness::fake::output_named as output;

    #[test]
    fn the_panel_sits_slightly_above_centre_but_below_the_bar() {
        let (bar, launcher) = (BarSettings::default(), LauncherSettings::default());
        // max(24 + 24, round(0.4 H - 230)).
        const CASES: &[(i32, i32)] = &[
            (1080, 202),
            (2160, 634),
            (768, 77),
            (600, 48),
            (400, 48),
            (1081, 202),
            (1082, 203),
        ];
        for &(height, want) in CASES {
            assert_eq!(panel_top(height, &bar, &launcher), want, "H {height}");
        }
    }

    #[test]
    fn the_panel_is_a_warm_overlay_with_the_keyboard() {
        let panel = layer(&output(Some("DP-1")), &ShellFile::default());
        assert_eq!(panel.namespace, "sill-launcher-DP-1");
        assert_eq!(panel.layer, LayerLevel::Overlay);
        assert_eq!(panel.anchor, Anchor::TOP);
        assert_eq!(
            (panel.size.width, panel.size.height),
            (LayerExtent::Px(680 + 360 + 84), LayerExtent::Px(460 + 66)),
            "the card, the pane's room beside it and the shadow's room"
        );
        assert_eq!(panel.margin.top, 202);
        assert_eq!(panel.keyboard, KeyboardMode::Exclusive);
        assert!(matches!(
            panel.blur,
            BlurSpec::Region(RegionSpec::Element { .. })
        ));
    }
}
