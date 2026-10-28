//! Night Shift's policy from `display.night_warmth`: where sill warms the monitors itself over
//! DDC/CI, casement's software night light is told to leave those alone, so they are not warmed
//! twice, and keeps warming the rest.

use sill_model::night_shift::NightShiftCommand;
use sill_settings::{NightWarmthMode, ShellFile};

use super::Policy;

/// Who warms the DDC monitors, from `display.night_warmth`.
#[derive(Debug, Clone, Copy)]
pub struct NightShiftBridge;

impl Policy for NightShiftBridge {
    type Command = NightShiftCommand;
    type Out = NightWarmthMode;

    fn of(&self, file: &ShellFile) -> NightWarmthMode {
        file.display.night_warmth
    }

    fn command(mode: NightWarmthMode) -> NightShiftCommand {
        NightShiftCommand::Backend(mode)
    }
}

#[cfg(test)]
mod tests {
    use sill_settings::{DisplaySettings, NightWarmthMode, ShellFile};

    use super::NightShiftBridge;
    use crate::policy::Policy;

    #[test]
    fn warming_over_ddc_is_the_hardware_backend() {
        let file = ShellFile {
            display: DisplaySettings {
                night_warmth: NightWarmthMode::Hardware,
                ..DisplaySettings::default()
            },
            ..ShellFile::default()
        };
        assert_eq!(NightShiftBridge.of(&file), NightWarmthMode::Hardware);
    }
}
