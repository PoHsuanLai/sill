//! The voice service on the settings' side: `voice.hold_to_talk` and `voice.consent` as the typed
//! [`VoiceParams`] the service gates a hold on, sent now and whenever the answer changes.

use sill_model::voice::{VoiceCommand, VoiceParams};
use sill_settings::{ShellFile, voice_params};

use super::Policy;

/// The voice service's gate from the settings.
#[derive(Debug, Clone, Copy)]
pub struct VoiceBridge;

impl Policy for VoiceBridge {
    type Command = VoiceCommand;
    type Out = VoiceParams;

    fn of(&self, file: &ShellFile) -> VoiceParams {
        voice_params(&file.voice)
    }

    fn command(out: VoiceParams) -> VoiceCommand {
        VoiceCommand::Params(out)
    }
}

#[cfg(test)]
mod tests {
    use sill_model::voice::{HoldToTalk, VoiceCommand, VoiceConsent, VoiceParams};
    use sill_settings::ShellFile;

    use super::VoiceBridge;
    use crate::policy::Policy;

    #[test]
    fn the_params_follow_the_keys_and_travel_as_the_services_command() {
        assert_eq!(
            VoiceBridge.of(&ShellFile::default()),
            VoiceParams::default()
        );
        let mut file = ShellFile::default();
        file.voice.hold_to_talk = HoldToTalk::Off;
        file.voice.consent = VoiceConsent::Declined;
        let out = VoiceBridge.of(&file);
        assert_eq!(
            (out.hold_to_talk, out.consent),
            (HoldToTalk::Off, VoiceConsent::Declined)
        );
        assert_eq!(VoiceBridge::command(out), VoiceCommand::Params(out));
    }
}
