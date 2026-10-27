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
        help = "The display under the pointer, or every display.",
        section = "Display",
        advanced
    )]
    pub brightness_keys_target: KeysTarget,
    #[settings(
        label = "Monitor speakers follow the volume",
        help = "Volume changes also set the monitor's own volume when it supports that.",
        section = "Display",
        advanced
    )]
    pub hardware_volume: HardwareVolume,
    #[settings(
        label = "Night warmth",
        help = "Warms external monitors by lowering their blue and green; undone when off.",
        section = "Display",
        advanced
    )]
    pub night_warmth: NightWarmthMode,
    #[settings(
        label = "Night warmth strength",
        help = "0 leaves white alone; 100 warms it the most.",
        section = "Display",
        range = "0..=100",
        unit = "%",
        advanced
    )]
    pub night_warmth_strength: Percent,
    #[settings(
        label = "Text rendering in other apps",
        help = "Auto sets smooth text for other apps, unless you've set your own.",
        section = "Display",
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
