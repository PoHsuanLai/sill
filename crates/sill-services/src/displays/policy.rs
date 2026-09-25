//! The `display.*` settings as this crate sees them. `sill-settings` owns the file and its
//! schema but brings `dioxus` with it, which this crate may not reach
//! (`scripts/check-boundary.sh`), so the daemon translates `DisplaySettings` into a
//! [`DisplayPolicy`] and sends it with `DisplaysCommand::Policy`.

use serde::{Deserialize, Serialize};

use super::identity::IdentityKey;
use super::scale::{DisplayScale, Ppi, ScaleRounding};
use crate::units::Percent;

/// Whether sill picks a new display's scale.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScaleMode {
    /// On first sight of a display, set the scale its density calls for.
    #[default]
    Auto,
    /// Never touch a scale; the compositor's settings rule.
    Manual,
}

/// One `display.scale_overrides` row: this display gets this scale.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ScaleOverride {
    pub display: IdentityKey,
    pub scale: DisplayScale,
}

impl ScaleOverride {
    /// `BNQ-8074-1f2e3d4c5b6a7980 = 1.75`; `None` for a row that does not read that way.
    pub fn parse(row: &str) -> Option<ScaleOverride> {
        let (key, scale) = row.split_once('=')?;
        let key = key.trim();
        (!key.is_empty() && !key.contains(char::is_whitespace)).then_some(())?;
        Some(ScaleOverride {
            display: IdentityKey(key.to_owned()),
            scale: DisplayScale::parse(scale)?,
        })
    }
}

/// Which outputs the brightness keys (and the OSD, and the control-center slider) act on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KeysTarget {
    /// The output under the pointer; the focused output when that is not known.
    #[default]
    PointerOutput,
    AllOutputs,
}

/// Whether the volume follows through to a monitor's own speakers over DDC/CI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HardwareVolume {
    Off,
    /// When the default sink is a monitor's HDMI/DisplayPort audio and the monitor answers VCP
    /// 0x62, its speaker volume and mute follow the software volume.
    #[default]
    WhenMonitorIsOutput,
}

/// Night warmth through the displays' own RGB gains.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum NightWarmth {
    #[default]
    Off,
    /// Warm every DDC display by this strength (0 = neutral, 100 = warmest).
    Hardware(Percent),
}

/// Whether sill writes its fontconfig snippet for apps that are not sill's.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FontRenderingMode {
    #[default]
    Auto,
    Off,
}

/// Every `display.*` decision the service makes, from settings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DisplayPolicy {
    pub scale: ScaleMode,
    pub scale_overrides: Vec<ScaleOverride>,
    pub density_target: Ppi,
    pub scale_rounding: ScaleRounding,
    pub keys_target: KeysTarget,
    pub hardware_volume: HardwareVolume,
    pub night_warmth: NightWarmth,
    pub font_rendering: FontRenderingMode,
}

impl Default for DisplayPolicy {
    fn default() -> Self {
        DisplayPolicy {
            scale: ScaleMode::Auto,
            scale_overrides: Vec::new(),
            density_target: Ppi::APPLE_27,
            scale_rounding: ScaleRounding::default(),
            keys_target: KeysTarget::PointerOutput,
            hardware_volume: HardwareVolume::WhenMonitorIsOutput,
            night_warmth: NightWarmth::Off,
            font_rendering: FontRenderingMode::Auto,
        }
    }
}

impl DisplayPolicy {
    /// The override for `display`, if a row names it (the last row wins).
    pub fn override_for(&self, display: &IdentityKey) -> Option<DisplayScale> {
        self.scale_overrides
            .iter()
            .rev()
            .find(|row| &row.display == display)
            .map(|row| row.scale)
    }
}

#[cfg(test)]
mod tests {
    use super::{DisplayPolicy, ScaleOverride};
    use crate::displays::identity::IdentityKey;
    use crate::displays::scale::DisplayScale;

    #[test]
    fn override_rows_parse_or_are_refused() {
        let row = |key: &str, scale| {
            Some(ScaleOverride {
                display: IdentityKey(key.to_owned()),
                scale: DisplayScale(scale),
            })
        };
        let cases: Vec<(&str, Option<ScaleOverride>)> = vec![
            ("BNQ-8074-00ff = 1.75", row("BNQ-8074-00ff", 175)),
            ("BNQ-8074-00ff=2", row("BNQ-8074-00ff", 200)),
            ("  DEL-a0b1-1 =1.5 ", row("DEL-a0b1-1", 150)),
            ("BNQ 8074 = 2", None),
            ("= 2", None),
            ("BNQ-8074-00ff", None),
            ("BNQ-8074-00ff = big", None),
        ];
        for (text, want) in cases {
            assert_eq!(ScaleOverride::parse(text), want, "{text:?}");
        }
    }

    #[test]
    fn the_last_override_for_a_display_wins() {
        let policy = DisplayPolicy {
            scale_overrides: ["A = 1.5", "B = 2", "A = 1.25"]
                .iter()
                .filter_map(|r| ScaleOverride::parse(r))
                .collect(),
            ..DisplayPolicy::default()
        };
        let key = |k: &str| IdentityKey(k.to_owned());
        assert_eq!(policy.override_for(&key("A")), Some(DisplayScale(125)));
        assert_eq!(policy.override_for(&key("B")), Some(DisplayScale(200)));
        assert_eq!(policy.override_for(&key("C")), None);
    }
}
