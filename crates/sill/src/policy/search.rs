//! The search runner's policy from `launcher.*` (files, web, emoji tone, currency). The service is
//! passive until the first, which is sent at start and again whenever the settings change it.

use std::time::Duration;

use sill_launcher::SkinTone;
use sill_model::results::{CurrencyPolicy, FilesChoice, LauncherPolicy, SearchCommand, WebPolicy};
use sill_settings::{
    CurrencyRates, EmojiSkinTone, FilesBackend, LauncherSettings, ShellFile, WebSearch,
};

use super::Policy;

/// The search runner's policy `l` makes.
pub fn search_policy_of(l: &LauncherSettings) -> LauncherPolicy {
    LauncherPolicy {
        files: match l.files_backend {
            FilesBackend::Auto => FilesChoice::Auto,
            FilesBackend::Tracker => FilesChoice::Tracker,
            FilesBackend::Fd => FilesChoice::Walk,
            FilesBackend::Off => FilesChoice::Off,
        },
        files_budget: Duration::from_millis(u64::from(l.files_budget_ms.0)),
        web: match l.web_search {
            WebSearch::On => WebPolicy::On {
                template: l.web_engine_url.clone(),
            },
            WebSearch::Off => WebPolicy::Off,
        },
        emoji_tone: match l.emoji_skin_tone {
            EmojiSkinTone::Default => SkinTone::Default,
            EmojiSkinTone::Light => SkinTone::Light,
            EmojiSkinTone::MediumLight => SkinTone::MediumLight,
            EmojiSkinTone::Medium => SkinTone::Medium,
            EmojiSkinTone::MediumDark => SkinTone::MediumDark,
            EmojiSkinTone::Dark => SkinTone::Dark,
        },
        currency: match l.currency {
            CurrencyRates::Off => CurrencyPolicy::Off,
            CurrencyRates::Ecb => CurrencyPolicy::Ecb,
        },
    }
}

/// The search runner's policy from `launcher.*`.
#[derive(Debug, Clone, Copy)]
pub struct SearchBridge;

impl Policy for SearchBridge {
    type Command = SearchCommand;
    type Out = LauncherPolicy;

    fn of(&self, file: &ShellFile) -> LauncherPolicy {
        search_policy_of(&file.launcher)
    }

    fn command(policy: LauncherPolicy) -> SearchCommand {
        SearchCommand::Policy(policy)
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use sill_launcher::SkinTone;
    use sill_model::results::{CurrencyPolicy, FilesChoice, WebPolicy};
    use sill_settings::{
        CurrencyRates, EmojiSkinTone, FilesBackend, LauncherSettings, Ms, WebSearch,
    };

    use super::search_policy_of;

    #[test]
    fn the_defaults_search_files_and_the_web_with_no_network_for_currency() {
        let search = search_policy_of(&LauncherSettings::default());
        assert_eq!(search.files, FilesChoice::Auto);
        assert_eq!(search.files_budget, Duration::from_millis(150));
        assert_eq!(
            search.web,
            WebPolicy::On {
                template: "https://duckduckgo.com/?q={query}".to_owned()
            }
        );
        assert_eq!(search.emoji_tone, SkinTone::Default);
        assert_eq!(
            search.currency,
            CurrencyPolicy::Off,
            "no network by default"
        );
    }

    #[test]
    fn every_setting_reaches_the_search_policy() {
        let l = LauncherSettings {
            files_backend: FilesBackend::Fd,
            files_budget_ms: Ms(400),
            emoji_skin_tone: EmojiSkinTone::MediumDark,
            web_search: WebSearch::Off,
            currency: CurrencyRates::Ecb,
            ..LauncherSettings::default()
        };
        let search = search_policy_of(&l);
        assert_eq!(search.files, FilesChoice::Walk);
        assert_eq!(search.files_budget, Duration::from_millis(400));
        assert_eq!(search.web, WebPolicy::Off);
        assert_eq!(search.emoji_tone, SkinTone::MediumDark);
        assert_eq!(search.currency, CurrencyPolicy::Ecb);
    }
}
