//! The clipboard service's policy from `launcher.*` (history on or off, its length, the largest
//! image). The service is passive until the first, which is sent at start and again whenever the
//! settings change it.

use sill_model::clipboard::{ClipPolicy, ClipboardCommand, EntryCap, HistoryMode, ImageCap};
use sill_settings::{ClipboardHistory, LauncherSettings, ShellFile};

use super::Policy;

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
    use sill_settings::{ClipboardHistory, Count, LauncherSettings};

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
            clipboard_history: ClipboardHistory::Off,
            clipboard_cap: Count(7),
            clipboard_image_max_mb: Count(0),
            ..LauncherSettings::default()
        };
        let clip = clip_policy_of(&l);
        assert_eq!(clip.mode, HistoryMode::Off);
        assert_eq!(clip.cap, EntryCap(7));
        assert_eq!(clip.images, ImageCap(0));
    }
}
