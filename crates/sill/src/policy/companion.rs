//! The companion service on the settings' side: `companion.double_tap_ms`,
//! `companion.app_wait_ms` and `companion.in_field` as the typed [`CompanionParams`] its
//! machines run on ([`sill_settings::companion_params`] holds each to its range), sent now and
//! whenever the answer changes.

use sill_model::companion::{CompanionCommand, CompanionParams};
use sill_settings::{ShellFile, companion_params};

use super::Policy;

/// The companion service's timing and summon place from the settings.
#[derive(Debug, Clone, Copy)]
pub struct CompanionBridge;

impl Policy for CompanionBridge {
    type Command = CompanionCommand;
    type Out = CompanionParams;

    fn of(&self, file: &ShellFile) -> CompanionParams {
        companion_params(&file.companion)
    }

    fn command(out: CompanionParams) -> CompanionCommand {
        CompanionCommand::Params(out)
    }
}

#[cfg(test)]
mod tests {
    use sill_model::companion::{CompanionCommand, CompanionParams, InField};
    use sill_model::units::Ms;
    use sill_settings::{Ms as KeyMs, ShellFile};

    use super::CompanionBridge;
    use crate::policy::Policy;

    #[test]
    fn the_params_follow_the_keys_and_travel_as_the_services_command() {
        let defaults = CompanionBridge.of(&ShellFile::default());
        assert_eq!(defaults, CompanionParams::default());
        let mut file = ShellFile::default();
        file.companion.double_tap_ms = KeyMs(500);
        file.companion.app_wait_ms = KeyMs(200);
        file.companion.in_field = InField::Launcher;
        let out = CompanionBridge.of(&file);
        assert_eq!(
            (out.tap.window, out.summon.app_wait, out.summon.in_field),
            (Ms(500), Ms(200), InField::Launcher)
        );
        assert_eq!(CompanionBridge::command(out), CompanionCommand::Params(out));
    }
}
