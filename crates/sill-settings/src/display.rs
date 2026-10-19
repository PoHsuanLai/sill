//! `display.*`: how sill treats displays (brightness and volume keys, night warmth, font
//! rendering for other apps, colour management). All Advanced for now (design/22 §5 has no
//! Displays page yet). There is no scale key: the compositor owns a display's scale.

use ds_core::word::Word;
use ds_settings::Percent;
use ds_settings::schema::Page;
use serde::{Deserialize, Serialize};
use sill_model::displays::{FontRenderingMode, HardwareVolume, KeysTarget};

/// `display.night_warmth`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, Word)]
#[serde(rename_all = "snake_case")]
#[word(case = snake)]
pub enum NightWarmthMode {
    #[default]
    Off,
    Hardware,
}

/// Displays.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ds_settings::SettingsSchema)]
#[serde(default)]
#[settings(file = "sill/settings.toml", domain = "display", page = Page::Appearance)]
pub struct DisplaySettings {
    #[settings(
        label = "Brightness keys adjust",
        help = "The display under the pointer (else the focused one), or every display at once.",
        section = "Brightness",
        advanced
    )]
    pub brightness_keys_target: KeysTarget,
    #[settings(
        label = "Monitor speakers follow the volume",
        help = "When sound plays through a monitor's HDMI or DisplayPort speakers and the monitor supports DDC/CI, volume changes also set the monitor's own volume and mute.",
        section = "Sound",
        advanced
    )]
    pub hardware_volume: HardwareVolume,
    #[settings(
        label = "Night warmth",
        help = "Hardware: warm external monitors by lowering their own green and blue gains over DDC/CI. Their gains are put back when this is turned off.",
        section = "Night warmth",
        advanced
    )]
    pub night_warmth: NightWarmthMode,
    #[settings(
        label = "Night warmth strength",
        help = "0 leaves white at 6500 K; 100 warms it to about 2700 K.",
        section = "Night warmth",
        range = "0..=100",
        unit = "%",
        advanced
    )]
    pub night_warmth_strength: Percent,
    #[settings(
        label = "Font rendering for other apps",
        help = "Auto: greyscale antialiasing, and hinting only below 150 ppi, written to ~/.config/fontconfig/conf.d/50-sill-rendering.conf unless a file of your own already sets them. Off removes sill's file.",
        section = "Text",
        advanced
    )]
    pub font_rendering: FontRenderingMode,
}

impl Default for DisplaySettings {
    fn default() -> Self {
        DisplaySettings {
            brightness_keys_target: KeysTarget::PointerOutput,
            hardware_volume: HardwareVolume::WhenMonitorIsOutput,
            night_warmth: NightWarmthMode::Off,
            night_warmth_strength: Percent(50),
            font_rendering: FontRenderingMode::Auto,
        }
    }
}
