//! The screenshots service's policy from `screenshot.folders` and `screenshot.tool`. Until the
//! first is sent the service stays passive and watches nothing.

use std::path::{Path, PathBuf};

use sill_model::screenshots::{ShotCommand, ShotPolicy, parse_folders, parse_tool};
use sill_settings::ShellFile;

use super::Policy;

/// The service's policy `file` makes (`parse_folders` against `home`, `parse_tool`).
pub fn policy_of(file: &ShellFile, home: &Path) -> ShotPolicy {
    ShotPolicy {
        folders: parse_folders(&file.screenshot.folders, home),
        tool: parse_tool(&file.screenshot.tool),
    }
}

/// The screenshots service's policy from the settings, its folders found against `home`.
#[derive(Debug, Clone)]
pub struct ScreenshotsBridge {
    pub home: PathBuf,
}

impl Policy for ScreenshotsBridge {
    type Command = ShotCommand;
    type Out = ShotPolicy;

    fn of(&self, file: &ShellFile) -> ShotPolicy {
        policy_of(file, &self.home)
    }

    fn command(policy: ShotPolicy) -> ShotCommand {
        ShotCommand::Configure(policy)
    }
}
#[cfg(test)]
mod tests {
    use sill_model::screenshots::{ShotFolders, ShotPolicy, ShotTool};
    use sill_settings::ShellFile;

    use super::policy_of;

    #[test]
    fn the_default_settings_watch_pictures_with_cosmics_tool() {
        assert_eq!(
            policy_of(&ShellFile::default(), std::path::Path::new("/home/p")),
            ShotPolicy {
                folders: ShotFolders::Auto,
                tool: ShotTool::Cosmic,
            }
        );
    }
}
