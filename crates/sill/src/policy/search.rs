//! The search runner's policy from `launcher.*` (files, web, emoji tone, currency). The service is
//! passive until the first, which is sent at start and again whenever the settings change it.

use std::time::Duration;

use sill_model::results::{LauncherPolicy, SearchCommand};
use sill_settings::{LauncherSettings, ShellFile};

use super::Policy;

/// How long one file search may take before its rows are dropped.
const FILES_BUDGET: Duration = Duration::from_millis(150);

/// The web search the fallback row opens, with `{query}` replaced by the typed text.
const WEB_SEARCH_URL: &str = "https://duckduckgo.com/?q={query}";

/// The search runner's policy `l` makes.
pub fn search_policy_of(l: &LauncherSettings) -> LauncherPolicy {
    LauncherPolicy {
        files_budget: FILES_BUDGET,
        web: l.web_search.is_on().then(|| WEB_SEARCH_URL.to_owned()),
        emoji_tone: l.emoji_skin_tone,
        currency: l.currency,
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
    use sill_model::results::CurrencyPolicy;
    use sill_settings::{LauncherSettings, OnOff};

    use super::search_policy_of;

    #[test]
    fn the_defaults_search_files_and_the_web_with_no_network_for_currency() {
        let search = search_policy_of(&LauncherSettings::default());
        assert_eq!(search.files_budget, Duration::from_millis(150));
        assert_eq!(
            search.web,
            Some("https://duckduckgo.com/?q={query}".to_owned())
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
            emoji_skin_tone: SkinTone::MediumDark,
            web_search: OnOff::Off,
            currency: CurrencyPolicy::Ecb,
            ..LauncherSettings::default()
        };
        let search = search_policy_of(&l);
        assert_eq!(search.web, None);
        assert_eq!(search.emoji_tone, SkinTone::MediumDark);
        assert_eq!(search.currency, CurrencyPolicy::Ecb);
    }
}
