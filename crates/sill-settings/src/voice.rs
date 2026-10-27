//! `voice.*`: hold-to-talk with the companion. `hold_to_talk` is the one control; `consent` is the
//! answer to the first-use sheet, state rather than a control, and `voiced` reads both from this
//! file on every hold (its `[voice]` table), so their forms are its contract
//! ([`VoiceConsent`]'s serde). The service reads them through [`params`].

use ds_settings::schema::Page;
use serde::{Deserialize, Serialize};
use sill_model::voice::VoiceParams;

pub use sill_model::voice::{HoldToTalk, VoiceConsent};

/// Hold-to-talk and the answer to its consent sheet.
#[derive(
    Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, ds_settings::SettingsSchema,
)]
#[serde(default)]
#[settings(file = "sill/settings.toml", domain = "voice", page = Page::Intelligence)]
pub struct VoiceSettings {
    #[settings(
        label = "Hold ⌘ to talk",
        help = "Listens while ⌘ is held and asks the assistant what it heard.",
        section = "Opening"
    )]
    pub hold_to_talk: HoldToTalk,
    /// The first-use sheet's answer: written by the sheet, never shown as a control.
    #[settings(skip)]
    pub consent: VoiceConsent,
}

/// What the voice service runs on for `settings`.
pub fn params(settings: &VoiceSettings) -> VoiceParams {
    VoiceParams {
        hold_to_talk: settings.hold_to_talk,
        consent: settings.consent,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::file::ShellFile;
    use prov::UnixSeconds;

    #[test]
    fn the_defaults_listen_once_consent_is_given_and_the_params_follow_the_keys() {
        let settings = VoiceSettings::default();
        assert_eq!(settings.hold_to_talk, HoldToTalk::HoldCommand);
        assert_eq!(settings.consent, VoiceConsent::NotAsked);
        assert_eq!(params(&settings), VoiceParams::default());
        let given = VoiceSettings {
            hold_to_talk: HoldToTalk::Off,
            consent: VoiceConsent::Given(UnixSeconds(7)),
        };
        assert_eq!(params(&given).consent, VoiceConsent::Given(UnixSeconds(7)));
        assert_eq!(params(&given).hold_to_talk, HoldToTalk::Off);
    }

    #[test]
    fn the_file_holds_exactly_the_keys_voiced_reads() {
        let file = ShellFile {
            voice: VoiceSettings {
                hold_to_talk: HoldToTalk::Off,
                consent: VoiceConsent::Given(UnixSeconds(1_760_000_000)),
            },
            ..ShellFile::default()
        };
        let text = toml::to_string(&file).unwrap_or_else(|e| panic!("{e}"));
        let table: toml::Table = text.parse().unwrap_or_else(|e| panic!("{e}"));
        let voice = table["voice"]
            .as_table()
            .unwrap_or_else(|| panic!("{text}"));
        assert_eq!(voice["hold_to_talk"].as_str(), Some("off"));
        assert_eq!(
            voice["consent"]["given"].as_integer(),
            Some(1_760_000_000),
            "{text}"
        );
        let declined = ShellFile {
            voice: VoiceSettings {
                consent: VoiceConsent::Declined,
                ..VoiceSettings::default()
            },
            ..ShellFile::default()
        };
        let text = toml::to_string(&declined).unwrap_or_else(|e| panic!("{e}"));
        let table: toml::Table = text.parse().unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(table["voice"]["consent"].as_str(), Some("declined"));
    }
}
