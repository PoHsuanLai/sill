//! The clipboard service's policy from `launcher.*` (history on or off; its length and the largest
//! image are constants). The service is passive until the first, which is sent at start and again whenever the
//! settings change it.

use sill_model::clipboard::{ClipPolicy, ClipboardCommand, EntryCap, ImageCap};
use sill_settings::{LauncherSettings, ShellFile};

use super::Policy;

/// Most history entries kept.
const ENTRY_CAP: u16 = 50;

/// The largest image kept, in bytes.
const IMAGE_MAX_BYTES: u64 = 8 << 20;

/// The clipboard service's policy `l` makes.
pub fn clip_policy_of(l: &LauncherSettings) -> ClipPolicy {
    ClipPolicy {
        mode: l.clipboard_history,
        cap: EntryCap(ENTRY_CAP),
        images: ImageCap(IMAGE_MAX_BYTES),
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
    use sill_model::clipboard::{EntryCap, HistoryMode, ImageCap};
    use sill_settings::LauncherSettings;

    use super::clip_policy_of;

    #[test]
    fn the_defaults_keep_history_in_memory() {
        let clip = clip_policy_of(&LauncherSettings::default());
        assert_eq!(clip.mode, HistoryMode::Memory);
        assert_eq!(clip.cap, EntryCap(50));
        assert_eq!(clip.images, ImageCap(8 << 20));
    }

    #[test]
    fn every_setting_reaches_the_clipboard_policy() {
        let l = LauncherSettings {
            clipboard_history: HistoryMode::Off,
            ..LauncherSettings::default()
        };
        let clip = clip_policy_of(&l);
        assert_eq!(clip.mode, HistoryMode::Off);
    }
}
