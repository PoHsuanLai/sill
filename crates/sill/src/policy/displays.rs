//! The displays service's policy: the `display.*` settings translated into the service's
//! [`DisplayPolicy`] (sill-services may not see sill-settings, which brings dioxus).

use sill_model::displays::{DisplayPolicy, DisplaysCommand, NightWarmth};

use super::Policy;
use sill_settings::{DisplaySettings, NightWarmthMode, ShellFile};

/// `display.*` as the service's policy.
pub fn policy_from(settings: &DisplaySettings) -> DisplayPolicy {
    DisplayPolicy {
        keys_target: settings.brightness_keys_target,
        hardware_volume: settings.hardware_volume,
        night_warmth: match settings.night_warmth {
            NightWarmthMode::Off => NightWarmth::Off,
            NightWarmthMode::Hardware => NightWarmth::Hardware(sill_model::units::Percent(
                u16::from(settings.night_warmth_strength.0),
            )),
        },
        font_rendering: settings.font_rendering,
    }
}

/// The displays service's policy from `display.*`.
#[derive(Debug, Clone, Copy)]
pub struct DisplaysBridge;

impl Policy for DisplaysBridge {
    type Command = DisplaysCommand;
    type Out = DisplayPolicy;

    fn of(&self, file: &ShellFile) -> DisplayPolicy {
        policy_from(&file.display)
    }

    fn command(policy: DisplayPolicy) -> DisplaysCommand {
        DisplaysCommand::Policy(policy)
    }
}

#[cfg(test)]
mod tests {
    use sill_model::displays::{
        DisplayPolicy, FontRenderingMode, HardwareVolume, KeysTarget, NightWarmth,
    };
    use sill_model::units::Percent;
    use sill_settings::{DisplaySettings, NightWarmthMode};

    use super::policy_from;

    #[test]
    fn the_default_settings_are_the_default_policy() {
        assert_eq!(
            policy_from(&DisplaySettings::default()),
            DisplayPolicy::default()
        );
    }

    #[test]
    fn every_setting_reaches_the_policy() {
        let settings = DisplaySettings {
            brightness_keys_target: KeysTarget::AllOutputs,
            hardware_volume: HardwareVolume::Off,
            night_warmth: NightWarmthMode::Hardware,
            night_warmth_strength: sill_settings::Percent(30),
            font_rendering: FontRenderingMode::Off,
        };
        assert_eq!(
            policy_from(&settings),
            DisplayPolicy {
                keys_target: KeysTarget::AllOutputs,
                hardware_volume: HardwareVolume::Off,
                night_warmth: NightWarmth::Hardware(Percent(30)),
                font_rendering: FontRenderingMode::Off,
            }
        );
    }
}
