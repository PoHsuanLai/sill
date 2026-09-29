//! The launcher v2 policy from `launcher.*`: the search runner's (files, web, emoji tone, currency)
//! and the clipboard service's (history on or off, its length, the largest image). Both services
//! are passive until the first one, which this sends at start and again whenever the settings
//! change it.

use std::time::Duration;

use sill_launcher::SkinTone;
use sill_services::clipboard::{ClipPolicy, ClipboardCommand, EntryCap, HistoryMode, ImageCap};
use sill_services::{CurrencyPolicy, FilesChoice, LauncherPolicy, SearchCommand, WebPolicy};
use sill_settings::{
    ClipboardHistory, CurrencyRates, EmojiSkinTone, FilesBackend, LauncherSettings, ShellFile,
    WebSearch,
};

use crate::policy::Policy;

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

/// The clipboard service's policy `l` makes.
pub fn clip_policy_of(l: &LauncherSettings) -> ClipPolicy {
    ClipPolicy {
        mode: match l.clipboard_history {
            ClipboardHistory::Memory => HistoryMode::Memory,
            ClipboardHistory::Off => HistoryMode::Off,
        },
        cap: EntryCap(l.clipboard_cap.0),
        images: ImageCap(u64::from(l.clipboard_image_max_mb.0) << 20),
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

/// The clipboard service's policy from `launcher.*`.
#[derive(Debug, Clone, Copy)]
pub struct ClipboardBridge;

impl Policy for ClipboardBridge {
    type Command = ClipboardCommand;
    type Out = ClipPolicy;

    fn of(&self, file: &ShellFile) -> ClipPolicy {
        clip_policy_of(&file.launcher)
    }

    fn command(policy: ClipPolicy) -> ClipboardCommand {
        ClipboardCommand::Policy(policy)
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use sill_launcher::SkinTone;
    use sill_services::clipboard::{EntryCap, HistoryMode, ImageCap};
    use sill_services::{CurrencyPolicy, FilesChoice, WebPolicy};
    use sill_settings::{
        ClipboardHistory, Count, CurrencyRates, EmojiSkinTone, FilesBackend, LauncherSettings, Ms,
        WebSearch,
    };

    use super::{clip_policy_of, search_policy_of};

    #[test]
    fn the_defaults_search_files_and_the_web_and_keep_history_in_memory() {
        let defaults = LauncherSettings::default();
        let search = search_policy_of(&defaults);
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
        let clip = clip_policy_of(&defaults);
        assert_eq!(clip.mode, HistoryMode::Memory);
        assert_eq!(clip.cap, EntryCap(50));
        assert_eq!(clip.images, ImageCap(8 << 20));
    }

    #[test]
    fn every_setting_reaches_its_policy() {
        let l = LauncherSettings {
            files_backend: FilesBackend::Fd,
            files_budget_ms: Ms(400),
            clipboard_history: ClipboardHistory::Off,
            clipboard_cap: Count(7),
            clipboard_image_max_mb: Count(0),
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
        let clip = clip_policy_of(&l);
        assert_eq!(clip.mode, HistoryMode::Off);
        assert_eq!(clip.cap, EntryCap(7));
        assert_eq!(clip.images, ImageCap(0));
    }
}
