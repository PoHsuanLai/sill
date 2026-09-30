//! The catcher's layer surface: Overlay over the whole output, no keyboard, no blur, input
//! everywhere the panel does not take it.

use shell_host::{
    Anchor, BlurSpec, ExclusiveZone, KeyboardMode, LayerConfig, LayerExtent, LayerLevel, LayerSize,
    Margin, OutputInfo, OutputTarget, Rect, RegionSpec, RoundedRect,
};
use sill_settings::{BarSettings, LauncherSettings};

use crate::links::PanelHit;
use crate::surface::namespace;
use crate::surfaces::launcher::layer::{CARD_INSET, panel_rect};

/// The catcher's input region once shown: the output less what the panel takes (`hit`: its
/// card, or the whole panel), so a click on the panel reaches the panel whichever of the two
/// Overlay surfaces the compositor stacks on top, and a click under a short card reaches the
/// catcher.
pub fn input(
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
pub fn layer(output: &OutputInfo) -> LayerConfig {
    LayerConfig {
        namespace: namespace("launcher-catcher", output),
        layer: LayerLevel::Overlay,
        anchor: Anchor::ALL,
        size: LayerSize {
            width: LayerExtent::Fill,
            height: LayerExtent::Fill,
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
    use super::{input, layer};
    use crate::harness::fake::output_named as output;
    use crate::links::PanelHit;
    use crate::surfaces::launcher::layer::panel_rect;
    use shell_host::{Anchor, BlurSpec, KeyboardMode, Rect, RegionSpec};
    use sill_settings::{BarSettings, LauncherSettings};

    #[test]
    fn the_catcher_covers_the_output_and_takes_no_keyboard_or_blur() {
        let catcher = layer(&output(Some("DP-1")));
        assert_eq!(catcher.namespace, "sill-launcher-catcher-DP-1");
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
        let RegionSpec::Rects(bands) = input(&out, &bar, &launcher, PanelHit::Whole) else {
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
        let RegionSpec::Rects(bands) = input(&out, &bar, &launcher, card) else {
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
