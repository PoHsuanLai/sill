//! The wallpaper service on the settings' side: `wallpaper.picture` as the typed
//! [`WallpaperPicture`], sent now and whenever it changes so the service re-prepares its images.

use std::path::PathBuf;

use sill_model::wallpaper::{WallpaperCommand, WallpaperPicture};
use sill_settings::ShellFile;

use super::Policy;

/// The wallpaper service's picture from the settings.
#[derive(Debug, Clone, Copy)]
pub struct WallpaperBridge;

impl Policy for WallpaperBridge {
    type Command = WallpaperCommand;
    type Out = WallpaperPicture;

    fn of(&self, file: &ShellFile) -> WallpaperPicture {
        match file.wallpaper.picture.trim() {
            "" => WallpaperPicture::Unset,
            path => WallpaperPicture::Chosen(PathBuf::from(path)),
        }
    }

    fn command(out: WallpaperPicture) -> WallpaperCommand {
        WallpaperCommand::Picture(out)
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use sill_model::wallpaper::{WallpaperCommand, WallpaperPicture};
    use sill_settings::ShellFile;

    use super::WallpaperBridge;
    use crate::policy::Policy;

    #[test]
    fn an_empty_key_is_unset_and_a_path_is_chosen() {
        let cases: &[(&str, &str, WallpaperPicture)] = &[
            ("empty", "", WallpaperPicture::Unset),
            ("blank", "  ", WallpaperPicture::Unset),
            (
                "a path",
                "/usr/share/backgrounds/dark.png",
                WallpaperPicture::Chosen(PathBuf::from("/usr/share/backgrounds/dark.png")),
            ),
        ];
        for (case, picture, want) in cases {
            let mut file = ShellFile::default();
            file.wallpaper.picture = (*picture).to_owned();
            let answer = WallpaperBridge.of(&file);
            assert_eq!(&answer, want, "{case}");
            assert_eq!(
                WallpaperBridge::command(answer.clone()),
                WallpaperCommand::Picture(answer),
                "{case}"
            );
        }
    }
}
