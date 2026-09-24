//! The launcher's two layer surfaces and the panel's vertical position (design/13 §13.5:
//! `top = max(bar_h + 24, round(0.4 H - 230))`, every number a `launcher.*` key).

use shell_host::{
    Anchor, BlurSpec, ElementId, ExclusiveZone, Extent, KeyboardMode, Layer, LayerConfig,
    LayerSize, Margin, OutputInfo, OutputTarget, RegionSpec,
};
use sill_settings::{BarSettings, LauncherSettings};

use crate::surfaces::naming::namespace;

/// The panel's fixed size (design/13 §13.3.9, settled by the plan).
pub const PANEL_WIDTH: u32 = 680;
pub const PANEL_HEIGHT: u32 = 460;

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

/// The panel: Overlay, anchored top only (centred horizontally), keyboard-exclusive, blurred
/// behind its `panel` element.
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
            width: Extent::Px(PANEL_WIDTH),
            height: Extent::Px(PANEL_HEIGHT),
        },
        exclusive: ExclusiveZone::Ignore,
        margin: Margin {
            top: panel_top(output.logical.height, bar, launcher),
            ..Margin::ZERO
        },
        keyboard: KeyboardMode::Exclusive,
        output: OutputTarget::Output(output.id),
        blur: BlurSpec::Region(RegionSpec::Element {
            id: ElementId::new("panel"),
            radius: 14,
            inflate: 0,
        }),
        input: RegionSpec::Whole,
    }
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
    use super::{catcher_layer, panel_layer, panel_top};
    use crate::surfaces::naming::tests::output;
    use shell_host::{Anchor, BlurSpec, Extent, KeyboardMode, Layer, RegionSpec};
    use sill_settings::{BarSettings, LauncherSettings};

    #[test]
    fn the_panel_sits_slightly_above_centre_but_below_the_bar() {
        let (bar, launcher) = (BarSettings::default(), LauncherSettings::default());
        // max(32 + 24, round(0.4 H - 230)).
        const CASES: &[(i32, i32)] = &[
            (1080, 202),
            (2160, 634),
            (768, 77),
            (600, 56),
            (400, 56),
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
            (Extent::Px(680), Extent::Px(460))
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
}
