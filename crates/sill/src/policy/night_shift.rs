//! Night Shift's policy from `display.night_warmth`: where sill warms the monitors itself over
//! DDC/CI, casement's software night light is told to leave those alone, so they are not warmed
//! twice, and keeps warming the rest.

use sill_model::night_shift::{Backend, NightShiftCommand};
use sill_settings::{NightWarmthMode, ShellFile};

use super::Policy;

/// Who warms the DDC monitors when `display.night_warmth` is `mode`.
pub fn backend_of(mode: NightWarmthMode) -> Backend {
    match mode {
        NightWarmthMode::Off => Backend::Software,
        NightWarmthMode::Hardware => Backend::Hardware,
    }
}

/// Night Shift's backend from `display.*`.
#[derive(Debug, Clone, Copy)]
pub struct NightShiftBridge;

impl Policy for NightShiftBridge {
    type Command = NightShiftCommand;
    type Out = Backend;

    fn of(&self, file: &ShellFile) -> Backend {
        backend_of(file.display.night_warmth)
    }

    fn command(backend: Backend) -> NightShiftCommand {
        NightShiftCommand::Backend(backend)
    }
}

#[cfg(test)]
mod tests {
    use sill_model::night_shift::Backend;
    use sill_settings::{DisplaySettings, NightWarmthMode, ShellFile};

    use super::{NightShiftBridge, backend_of};
    use crate::policy::Policy;

    #[test]
    fn warming_over_ddc_is_the_hardware_backend() {
        assert_eq!(backend_of(NightWarmthMode::Off), Backend::Software);
        assert_eq!(backend_of(NightWarmthMode::Hardware), Backend::Hardware);
        let file = ShellFile {
            display: DisplaySettings {
                night_warmth: NightWarmthMode::Hardware,
                ..DisplaySettings::default()
            },
            ..ShellFile::default()
        };
        assert_eq!(NightShiftBridge.of(&file), Backend::Hardware);
    }
}
