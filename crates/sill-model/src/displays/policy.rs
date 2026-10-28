//! The `display.*` settings as this crate sees them. `sill-settings` owns the file and its
//! schema but brings `dioxus` with it, which this crate may not reach
//! (`scripts/check-boundary.sh`), so the daemon translates `DisplaySettings` into a
//! [`DisplayPolicy`] and sends it with `DisplaysCommand::Policy`.

use ds_core::word::Word;
use serde::{Deserialize, Serialize};

use crate::units::Percent;

/// Which outputs the brightness keys (and the OSD, and the control-center slider) act on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, Word)]
#[serde(rename_all = "snake_case")]
#[word(case = snake)]
pub enum KeysTarget {
    /// The output under the pointer; the focused output when that is not known.
    #[default]
    PointerOutput,
    AllOutputs,
}

/// Whether the volume follows through to a monitor's own speakers over DDC/CI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, Word)]
#[serde(rename_all = "snake_case")]
#[word(case = snake)]
pub enum HardwareVolume {
    Off,
    /// When the default sink is a monitor's HDMI/DisplayPort audio and the monitor answers VCP
    /// 0x62, its speaker volume and mute follow the software volume.
    #[default]
    WhenMonitorIsOutput,
}

/// `display.night_warmth`: night warmth through the displays' own RGB gains.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, Word)]
#[serde(rename_all = "snake_case")]
#[word(case = snake)]
pub enum NightWarmthMode {
    #[default]
    Off,
    /// Warm every DDC display by `night_warmth_strength`.
    Hardware,
}

/// Whether sill writes its fontconfig snippet for apps that are not sill's.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, Word)]
#[serde(rename_all = "snake_case")]
#[word(case = snake)]
pub enum FontRenderingMode {
    #[default]
    Auto,
    Off,
}

/// Every `display.*` decision the service makes, from settings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DisplayPolicy {
    pub keys_target: KeysTarget,
    pub hardware_volume: HardwareVolume,
    pub night_warmth: NightWarmthMode,
    /// How far `Hardware` warms: 0 is neutral, 100 the warmest.
    pub night_warmth_strength: Percent,
    pub font_rendering: FontRenderingMode,
}

impl Default for DisplayPolicy {
    fn default() -> Self {
        DisplayPolicy {
            keys_target: KeysTarget::PointerOutput,
            hardware_volume: HardwareVolume::WhenMonitorIsOutput,
            night_warmth: NightWarmthMode::Off,
            night_warmth_strength: Percent(50),
            font_rendering: FontRenderingMode::Auto,
        }
    }
}

impl DisplayPolicy {
    /// The strength to warm the DDC displays to, or `None` when sill leaves them alone.
    pub fn warmth(&self) -> Option<Percent> {
        match self.night_warmth {
            NightWarmthMode::Hardware => Some(self.night_warmth_strength),
            NightWarmthMode::Off => None,
        }
    }
}
