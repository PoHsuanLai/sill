//! `display.*`: how sill treats displays (scale on first sight, brightness and volume keys,
//! night warmth, font rendering for other apps, colour management). All Advanced for now
//! (design/22 §5 has no Displays page yet); the rows are listed in sill FINDINGS "Displays"
//! until quire's design/22 gains them.
//!
//! `display.scale` is a two-word switch plus a separate list of per-display rows rather than one
//! key that is either `auto` or a table: the schema derive describes a key by one closed kind,
//! and a key that changes kind with its value cannot be drawn by one widget. The rows are one
//! text key, `;`-separated, because the derive's `Vec<_>` shape needs an enum element and has
//! no list-of-text kind yet (a quire gap, sill FINDINGS "Displays").

use ds_settings::schema::Page;
use ds_settings::{Count, Percent};
use serde::{Deserialize, Serialize};

/// `display.scale`.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    Default,
    Serialize,
    Deserialize,
    ds_settings::SettingsSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum DisplayScaleMode {
    #[default]
    Auto,
    Manual,
}

/// `display.brightness_keys_target`.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    Default,
    Serialize,
    Deserialize,
    ds_settings::SettingsSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum BrightnessKeysTarget {
    #[default]
    PointerOutput,
    AllOutputs,
}

/// `display.hardware_volume`.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    Default,
    Serialize,
    Deserialize,
    ds_settings::SettingsSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum HardwareVolumeMode {
    Off,
    #[default]
    WhenMonitorIsOutput,
}

/// `display.night_warmth`.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    Default,
    Serialize,
    Deserialize,
    ds_settings::SettingsSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum NightWarmthMode {
    #[default]
    Off,
    Hardware,
}

/// `display.font_rendering`.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    Default,
    Serialize,
    Deserialize,
    ds_settings::SettingsSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum FontRenderingSetting {
    #[default]
    Auto,
    Off,
}

/// `display.color_management`.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    Default,
    Serialize,
    Deserialize,
    ds_settings::SettingsSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum ColorManagement {
    #[default]
    Auto,
    Off,
}

/// Displays.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ds_settings::SettingsSchema)]
#[serde(default)]
#[settings(file = "sill/settings.toml", domain = "display", page = Page::Appearance)]
pub struct DisplaySettings {
    #[settings(
        label = "Choose display scale automatically",
        help = "Auto: the first time a display is connected, sill sets the scale its size and resolution call for (about 109 points per inch), and never changes it again. Manual: sill never changes a scale.",
        section = "Displays",
        advanced
    )]
    pub scale: DisplayScaleMode,
    #[settings(
        label = "Scale per display",
        help = "`<display> = <scale>` pairs separated by semicolons, e.g. `BNQ-8074-1f2e3d4c5b6a7980 = 1.75`; the display names are in ~/.local/state/sill/displays.json. A pair is applied when it is added or changed, even under Manual.",
        section = "Displays",
        advanced
    )]
    pub scale_overrides: String,
    #[settings(
        label = "Target density",
        help = "The points per inch the automatic scale aims for. 109 is a 27-inch 2560 x 1440 display; lower makes everything larger.",
        section = "Displays",
        range = "72..=220",
        unit = "ppi",
        advanced
    )]
    pub density_target_ppi: Count,
    #[settings(
        label = "Brightness keys adjust",
        help = "The display under the pointer (else the focused one), or every display at once.",
        section = "Brightness",
        advanced
    )]
    pub brightness_keys_target: BrightnessKeysTarget,
    #[settings(
        label = "Monitor speakers follow the volume",
        help = "When sound plays through a monitor's HDMI or DisplayPort speakers and the monitor supports DDC/CI, volume changes also set the monitor's own volume and mute.",
        section = "Sound",
        advanced
    )]
    pub hardware_volume: HardwareVolumeMode,
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
    pub font_rendering: FontRenderingSetting,
    #[settings(
        label = "Colour management",
        help = "Auto: use each display's reported gamut once the compositor supports colour management. Stored only for now.",
        section = "Colour",
        advanced
    )]
    pub color_management: ColorManagement,
    /// Keys this build does not know, kept for the next write.
    #[serde(flatten)]
    #[settings(skip)]
    pub extra: toml::Table,
}

impl Default for DisplaySettings {
    fn default() -> Self {
        DisplaySettings {
            scale: DisplayScaleMode::Auto,
            scale_overrides: String::new(),
            density_target_ppi: Count(109),
            brightness_keys_target: BrightnessKeysTarget::PointerOutput,
            hardware_volume: HardwareVolumeMode::WhenMonitorIsOutput,
            night_warmth: NightWarmthMode::Off,
            night_warmth_strength: Percent(50),
            font_rendering: FontRenderingSetting::Auto,
            color_management: ColorManagement::Auto,
            extra: toml::Table::new(),
        }
    }
}
