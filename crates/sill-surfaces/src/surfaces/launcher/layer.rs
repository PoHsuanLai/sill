//! The launcher's two layer surfaces and the panel's vertical position (design/13 §13.5:
//! `top = max(bar_h + 24, round(0.4 H - 230))`, every number a `launcher.*` key).

use shell_host::{
    Anchor, BlurSpec, ExclusiveZone, Extent, KeyboardMode, Layer, LayerConfig, LayerSize, Margin,
    OutputInfo, OutputTarget, Rect, RegionSpec, RoundedRect,
};
use sill_settings::{BarSettings, LauncherSettings};

use crate::surfaces::launcher::hit::{PanelHit, card_region, panel_input};
use crate::surfaces::launcher::preview::PREVIEW_WIDTH_PX;
use crate::surfaces::naming::namespace;

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

/// The panel's top edge on an output `output_height` tall.
pub fn panel_top(output_height: i32, bar: &BarSettings, launcher: &LauncherSettings) -> i32 {
    let below_bar = i32::from(bar.height_px.0) + i32::from(launcher.top_bar_gap_px.0);
    let fraction = i64::from(launcher.top_vertical_fraction.0);
    // round(fraction/1000 x H) in integers: add half the divisor before dividing.
    let scaled = (fraction * i64::from(output_height) + 500).div_euclid(1000);
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

/// The panel: Overlay, anchored top only (centred horizontally), keyboard-exclusive, blurred
/// behind the palette card ([`CARD_ID`]). Its input starts as the whole panel; the panel narrows
/// it to the card once it has measured one (`hit::PanelHit`).
pub fn panel_layer(
    output: &OutputInfo,
    bar: &BarSettings,
    launcher: &LauncherSettings,
) -> LayerConfig {
    LayerConfig {
        namespace: namespace("launcher", output),
        layer: Layer::Overlay,
        anchor: Anchor::TOP,
        size: LayerSize {
            width: Extent::Px(PANEL_ROOM_WIDTH + 2 * SHADOW_SIDE),
            height: Extent::Px(PANEL_HEIGHT + SHADOW_BELOW),
        },
        exclusive: ExclusiveZone::Ignore,
        margin: Margin {
            top: panel_top(output.logical.height, bar, launcher),
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

/// The catcher's input region once shown: the output less what the panel takes (`hit`: its
/// card, or the whole panel), so a click on the panel reaches the panel whichever of the two
/// Overlay surfaces the compositor stacks on top, and a click under a short card reaches the
/// catcher.
pub fn catcher_input(
    output: &OutputInfo,
    bar: &BarSettings,
    launcher: &LauncherSettings,
    hit: PanelHit,
) -> RegionSpec {
    let (w, h) = (output.logical.width, output.logical.height);
    let whole = panel_rect(output, bar, launcher);
    let inset = i32::try_from(CARD_INSET).unwrap_or(0);
    let panel = match hit {
        PanelHit::Whole => whole,
        // The card alone (no pane beside it): centred in the room, as tall as it measured.
        PanelHit::Card { height } => Rect {
            x: whole.x + inset,
            width: whole.width - 2 * inset,
            height: height.clamp(1, whole.height),
            ..whole
        },
    };
    let (right, bottom) = (panel.x + panel.width, panel.y + panel.height);
    let bands = [
        Rect {
            x: 0,
            y: 0,
            width: w,
            height: panel.y,
        },
        Rect {
            x: 0,
            y: bottom,
            width: w,
            height: h - bottom,
        },
        Rect {
            x: 0,
            y: panel.y,
            width: panel.x,
            height: panel.height,
        },
        Rect {
            x: right,
            y: panel.y,
            width: w - right,
            height: panel.height,
        },
    ];
    RegionSpec::Rects(
        bands
            .into_iter()
            .filter(|band| band.width > 0 && band.height > 0)
            .map(|rect| RoundedRect { rect, radius: 0 })
            .collect(),
    )
}

/// The catcher: Overlay, every edge, no keyboard, no blur, input everywhere.
pub fn catcher_layer(output: &OutputInfo) -> LayerConfig {
    LayerConfig {
        namespace: namespace("launcher-catcher", output),
        layer: Layer::Overlay,
        anchor: Anchor::ALL,
        size: LayerSize {
            width: Extent::Fill,
            height: Extent::Fill,
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
    use super::{catcher_input, catcher_layer, panel_layer, panel_rect, panel_top};
    use crate::surfaces::launcher::hit::PanelHit;
    use crate::surfaces::naming::tests::output;
    use shell_host::{Anchor, BlurSpec, Extent, KeyboardMode, Layer, Rect, RegionSpec};
    use sill_settings::{BarSettings, LauncherSettings};

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
    fn panel_and_catcher_are_warm_overlays() {
        let out = output(Some("DP-1"));
        let panel = panel_layer(&out, &BarSettings::default(), &LauncherSettings::default());
        assert_eq!(panel.namespace, "sill-launcher-DP-1");
        assert_eq!(panel.layer, Layer::Overlay);
        assert_eq!(panel.anchor, Anchor::TOP);
        assert_eq!(
            (panel.size.width, panel.size.height),
            (Extent::Px(680 + 360 + 84), Extent::Px(460 + 66)),
            "the card, the pane's room beside it and the shadow's room"
        );
        assert_eq!(panel.margin.top, 202);
        assert_eq!(panel.keyboard, KeyboardMode::Exclusive);
        assert!(matches!(
            panel.blur,
            BlurSpec::Region(RegionSpec::Element { .. })
        ));
        let catcher = catcher_layer(&out);
        assert_eq!(catcher.anchor, Anchor::ALL);
        assert_eq!(
            catcher.blur,
            BlurSpec::None,
            "no blur on the catcher (plan shell risks)"
        );
        assert_eq!(catcher.input, RegionSpec::Whole);
        assert_eq!(catcher.keyboard, KeyboardMode::None);
    }

    #[test]
    fn the_catcher_takes_every_click_but_the_panels() {
        let (bar, launcher) = (BarSettings::default(), LauncherSettings::default());
        let out = output(Some("DP-1"));
        let panel = panel_rect(&out, &bar, &launcher);
        assert_eq!(
            panel,
            Rect {
                x: 440,
                y: 202,
                width: 1040,
                height: 460
            },
            "the card's and the pane's room, centred on 1920 x 1080"
        );
        let RegionSpec::Rects(bands) = catcher_input(&out, &bar, &launcher, PanelHit::Whole) else {
            panic!("the catcher's region is a set of rects");
        };
        let area: i64 = bands
            .iter()
            .map(|b| i64::from(b.rect.width) * i64::from(b.rect.height))
            .sum();
        assert_eq!(area, 1920 * 1080 - 1040 * 460, "everything but the panel");
        let inside = |x: i32, y: i32| {
            bands.iter().any(|b| {
                x >= b.rect.x
                    && x < b.rect.x + b.rect.width
                    && y >= b.rect.y
                    && y < b.rect.y + b.rect.height
            })
        };
        assert!(inside(10, 10) && inside(439, 300) && inside(1480, 300) && inside(900, 662));
        assert!(!inside(440, 202) && !inside(1479, 661) && !inside(960, 400));
        // A 140 px card: the catcher takes everything under it too.
        let card = PanelHit::Card { height: 140 };
        let RegionSpec::Rects(bands) = catcher_input(&out, &bar, &launcher, card) else {
            panic!("the catcher's region is a set of rects");
        };
        let inside = |x: i32, y: i32| {
            bands.iter().any(|b| {
                x >= b.rect.x
                    && x < b.rect.x + b.rect.width
                    && y >= b.rect.y
                    && y < b.rect.y + b.rect.height
            })
        };
        assert!(!inside(620, 202) && !inside(1299, 341), "the card");
        assert!(inside(960, 342) && inside(960, 600), "under the card");
        assert!(
            inside(619, 300) && inside(1300, 300),
            "beside the card: the pane's room"
        );
    }
}
