//! The bar's layer surface (design/20 §1.1): Top, anchored top, left and right, the token
//! height (`--shell-bar-h`), reserving it, no keyboard, blurred and hit-tested whole. Under `bar.autohide` modes
//! that hide it on the desktop (`always`, `desktop_only`) it is the same surface at rest hidden:
//! nothing reserved, no blur, and only the thin strip at the top edge takes the pointer
//! (`reveal.rs` moves it from there).

use shell_host::{
    Anchor, BlurSpec, ExclusiveZone, KeyboardMode, LayerConfig, LayerExtent, LayerLevel, LayerSize,
    Margin, OutputInfo, OutputTarget, RegionSpec,
};
use sill_settings::MenuBarHiding;

use super::reveal::{rest_phase, strip_input};
use super::tokens::BarTokens;
use sill_ui_kit::autohide::Phase;
use sill_ui_kit::surface::namespace;

/// The bar's layer configuration on `output`, resting under `mode` (`reveal::mode_on`).
pub(super) fn layer(output: &OutputInfo, mode: MenuBarHiding, tokens: &BarTokens) -> LayerConfig {
    let height = u32::from(tokens.height);
    let phase = rest_phase(mode);
    LayerConfig {
        namespace: namespace("bar", output),
        layer: LayerLevel::Top,
        anchor: Anchor::TOP | Anchor::LEFT | Anchor::RIGHT,
        size: LayerSize {
            width: LayerExtent::Fill,
            height: LayerExtent::Px(height),
        },
        exclusive: match phase {
            Phase::Fixed => ExclusiveZone::Reserve(height),
            _ => ExclusiveZone::Reserve(0),
        },
        margin: Margin::ZERO,
        keyboard: KeyboardMode::None,
        output: OutputTarget::Output(output.id),
        blur: match phase {
            Phase::Fixed => BlurSpec::Region(RegionSpec::Whole),
            _ => BlurSpec::None,
        },
        input: match phase {
            Phase::Fixed => RegionSpec::Whole,
            _ => strip_input(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::layer;
    use crate::bar::tokens::BarTokens;
    use shell_host::{
        Anchor, BlurSpec, ExclusiveZone, KeyboardMode, LayerExtent, LayerLevel, OutputId,
        OutputTarget, RegionSpec,
    };
    use sill_settings::MenuBarHiding;
    use sill_ui_kit::harness::fake::output_named as output;

    #[test]
    fn the_bar_reserves_its_token_height() {
        let height = u32::from(BarTokens::default().height);
        let config = layer(
            &output(Some("DP-1")),
            MenuBarHiding::default(),
            &BarTokens::default(),
        );
        assert_eq!(config.namespace, "sill-bar-DP-1");
        assert_eq!(config.layer, LayerLevel::Top);
        assert_eq!(config.anchor, Anchor::TOP | Anchor::LEFT | Anchor::RIGHT);
        assert_eq!(config.size.width, LayerExtent::Fill);
        assert_eq!(config.size.height, LayerExtent::Px(height));
        assert_eq!(config.exclusive, ExclusiveZone::Reserve(height));
        assert_eq!(config.keyboard, KeyboardMode::None);
        assert_eq!(config.output, OutputTarget::Output(OutputId(7)));
        assert_eq!(config.input, RegionSpec::Whole);
    }

    #[test]
    fn the_modes_that_hide_it_on_the_desktop_reserve_nothing_and_leave_a_strip() {
        // (mode, reserved, blurred, input is the whole bar)
        let cases = [
            (MenuBarHiding::Always, 0, false, false),
            (MenuBarHiding::DesktopOnly, 0, false, false),
            (MenuBarHiding::FullScreenOnly, 24, true, true),
            (MenuBarHiding::Never, 24, true, true),
        ];
        for (mode, reserved, blurred, whole) in cases {
            let config = layer(&output(Some("DP-1")), mode, &BarTokens::default());
            assert_eq!(
                config.exclusive,
                ExclusiveZone::Reserve(reserved),
                "{mode:?}"
            );
            assert_eq!(config.size.height, LayerExtent::Px(24), "{mode:?}");
            assert_eq!(config.layer, LayerLevel::Top, "{mode:?}");
            assert_eq!(
                matches!(config.blur, BlurSpec::Region(_)),
                blurred,
                "{mode:?}"
            );
            assert_eq!(config.input == RegionSpec::Whole, whole, "{mode:?}");
        }
    }
}
