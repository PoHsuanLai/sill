//! The daemon's half of the displays service: the `display.*` settings translated into the
//! service's [`DisplayPolicy`] (sill-services may not see sill-settings, which brings dioxus),
//! and outputs changes forwarded as rescans.

use shell_host::HostEvent;
use sill_model::displays::{
    DisplayPolicy, DisplaysCommand, FontRenderingMode, HardwareVolume, KeysTarget, NightWarmth,
    Ppi, ScaleMode, ScaleOverride, ScaleRounding,
};
use sill_surfaces::services::Services;

use crate::daemon::msg::ShellMsg;
use crate::policy::Policy;
use sill_settings::{
    BrightnessKeysTarget, DisplayScaleMode, DisplaySettings, FontRenderingSetting,
    HardwareVolumeMode, NightWarmthMode, ScaleRounding as ScaleRoundingSetting, ShellFile,
};

/// `display.*` as the service's policy.
pub fn policy_from(settings: &DisplaySettings) -> DisplayPolicy {
    DisplayPolicy {
        scale: match settings.scale {
            DisplayScaleMode::Auto => ScaleMode::Auto,
            DisplayScaleMode::Manual => ScaleMode::Manual,
        },
        scale_overrides: settings
            .scale_overrides
            .split(';')
            .filter_map(ScaleOverride::parse)
            .collect(),
        density_target: Ppi(settings.density_target_ppi.0),
        scale_rounding: match settings.scale_rounding {
            ScaleRoundingSetting::PreferWhole => ScaleRounding::PreferWhole,
            ScaleRoundingSetting::Nearest => ScaleRounding::Nearest,
        },
        keys_target: match settings.brightness_keys_target {
            BrightnessKeysTarget::PointerOutput => KeysTarget::PointerOutput,
            BrightnessKeysTarget::AllOutputs => KeysTarget::AllOutputs,
        },
        hardware_volume: match settings.hardware_volume {
            HardwareVolumeMode::Off => HardwareVolume::Off,
            HardwareVolumeMode::WhenMonitorIsOutput => HardwareVolume::WhenMonitorIsOutput,
        },
        night_warmth: match settings.night_warmth {
            NightWarmthMode::Off => NightWarmth::Off,
            NightWarmthMode::Hardware => NightWarmth::Hardware(sill_model::units::Percent(
                u16::from(settings.night_warmth_strength.0),
            )),
        },
        font_rendering: match settings.font_rendering {
            FontRenderingSetting::Auto => FontRenderingMode::Auto,
            FontRenderingSetting::Off => FontRenderingMode::Off,
        },
    }
}

/// The displays service's policy from `display.*`.
#[derive(Debug, Clone, Copy)]
pub struct Bridge;

impl Policy for Bridge {
    type Command = DisplaysCommand;
    type Out = DisplayPolicy;

    fn of(&self, file: &ShellFile) -> DisplayPolicy {
        policy_from(&file.display)
    }

    fn command(policy: DisplayPolicy) -> DisplaysCommand {
        DisplaysCommand::Policy(policy)
    }
}

/// The host events the displays service follows: outputs coming, going or changing (a rescan).
pub fn on_host_event(services: &Services, event: &HostEvent<ShellMsg>) {
    match event {
        HostEvent::Started
        | HostEvent::OutputAdded(_)
        | HostEvent::OutputChanged(_)
        | HostEvent::OutputRemoved(_) => {
            let _ = services.displays.control.send(DisplaysCommand::Rescan);
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use sill_model::displays::ScaleOverride;
    use sill_model::displays::{
        DisplayPolicy, FontRenderingMode, HardwareVolume, KeysTarget, NightWarmth, Ppi, ScaleMode,
        ScaleRounding,
    };
    use sill_model::units::Percent;
    use sill_settings::{
        BrightnessKeysTarget, DisplayScaleMode, DisplaySettings, FontRenderingSetting,
        HardwareVolumeMode, NightWarmthMode, ScaleRounding as ScaleRoundingSetting,
    };

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
            scale: DisplayScaleMode::Manual,
            scale_overrides: "BNQ-8074-00ff = 1.75; nonsense; DEL-a0b1-1=2".to_owned(),
            density_target_ppi: sill_settings::Count(120),
            scale_rounding: ScaleRoundingSetting::Nearest,
            brightness_keys_target: BrightnessKeysTarget::AllOutputs,
            hardware_volume: HardwareVolumeMode::Off,
            night_warmth: NightWarmthMode::Hardware,
            night_warmth_strength: sill_settings::Percent(30),
            font_rendering: FontRenderingSetting::Off,
        };
        assert_eq!(
            policy_from(&settings),
            DisplayPolicy {
                scale: ScaleMode::Manual,
                scale_overrides: ["BNQ-8074-00ff = 1.75", "DEL-a0b1-1=2"]
                    .iter()
                    .filter_map(|r| ScaleOverride::parse(r))
                    .collect(),
                density_target: Ppi(120),
                scale_rounding: ScaleRounding::Nearest,
                keys_target: KeysTarget::AllOutputs,
                hardware_volume: HardwareVolume::Off,
                night_warmth: NightWarmth::Hardware(Percent(30)),
                font_rendering: FontRenderingMode::Off,
            }
        );
    }
}
