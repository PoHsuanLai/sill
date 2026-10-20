//! The layer configuration of the twelve full-output sheets, from their presets: one table in
//! place of a test per surface.

use shell_host::{
    Anchor, BlurSpec, ElementId, ExclusiveZone, KeyboardMode, LayerConfig, LayerExtent, LayerLevel,
    RegionSpec,
};
use sill_overlays::{
    AccountsSheet, CaptureSheet, ColorSampler, ConfirmSheet, DisplayTrialSheet, ForceQuit,
    PermissionSheet, PolkitPrompt, PowerMenu, SplitPicker, Switcher, VoiceConsentSheet,
};
use sill_settings::ShellFile;
use sill_ui_kit::harness::fake::output_named;
use sill_ui_kit::panel::PANEL_RADIUS;
use sill_ui_kit::surface::Surface;

/// What a surface's blur and input are: the panel element at a radius, or nothing / everything.
#[derive(Debug, Clone, Copy)]
enum Region {
    None,
    Whole,
    Panel(i32),
}

fn layer_of<S: Surface<Slot = ()>>() -> LayerConfig {
    S::layer(&output_named(Some("DP-1")), &ShellFile::default(), ())
}

/// (surface, its name, blur, input).
type Case = (LayerConfig, &'static str, Region, Region);

#[test]
fn the_sheets_are_keyboard_exclusive_full_output_overlays() {
    let sheet = Region::Panel(PANEL_RADIUS);
    let cases: [Case; 12] = [
        (layer_of::<ForceQuit>(), "force-quit", sheet, Region::Whole),
        (
            layer_of::<DisplayTrialSheet>(),
            "display-trial",
            sheet,
            Region::Whole,
        ),
        (layer_of::<ConfirmSheet>(), "confirm", sheet, Region::Whole),
        (
            layer_of::<PermissionSheet>(),
            "permission",
            sheet,
            Region::Whole,
        ),
        (
            layer_of::<VoiceConsentSheet>(),
            "voice-consent",
            sheet,
            Region::Whole,
        ),
        (layer_of::<CaptureSheet>(), "capture", sheet, Region::Whole),
        (layer_of::<PolkitPrompt>(), "polkit", sheet, Region::Whole),
        (layer_of::<PowerMenu>(), "power-menu", sheet, Region::Whole),
        // The switcher's panel is `Material::Osd`'s 18, and takes input only on itself.
        (
            layer_of::<Switcher>(),
            "switcher",
            Region::Panel(18),
            Region::Panel(18),
        ),
        (
            layer_of::<SplitPicker>(),
            "split-picker",
            Region::None,
            Region::Whole,
        ),
        (
            layer_of::<ColorSampler>(),
            "eyedropper",
            Region::None,
            Region::Whole,
        ),
        (
            layer_of::<AccountsSheet>(),
            "accounts-sheet",
            Region::None,
            Region::Whole,
        ),
    ];
    for (layer, name, blur, input) in cases {
        let panel = format!("{name}-panel");
        let region = |radius| RegionSpec::Element {
            id: ElementId::new(&panel),
            radius,
            inflate: 0,
        };
        assert_eq!(layer.namespace, format!("sill-{name}-DP-1"), "{name}");
        assert_eq!(layer.layer, LayerLevel::Overlay, "{name}");
        assert_eq!(layer.anchor, Anchor::ALL, "{name}");
        assert_eq!(
            (layer.size.width, layer.size.height),
            (LayerExtent::Fill, LayerExtent::Fill),
            "{name}"
        );
        assert_eq!(layer.exclusive, ExclusiveZone::Ignore, "{name}");
        assert_eq!(layer.keyboard, KeyboardMode::Exclusive, "{name}");
        assert_eq!(
            layer.blur,
            match blur {
                Region::None => BlurSpec::None,
                Region::Whole => unreachable!("a blur is a panel or nothing"),
                Region::Panel(radius) => BlurSpec::Region(region(radius)),
            },
            "{name}: blur"
        );
        assert_eq!(
            layer.input,
            match input {
                Region::None => RegionSpec::Empty,
                Region::Whole => RegionSpec::Whole,
                Region::Panel(radius) => region(radius),
            },
            "{name}: input"
        );
    }
}
