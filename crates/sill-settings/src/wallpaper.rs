//! `wallpaper.*`: the picture on the desktop. One key, drawn on the Wallpaper page as a grid of
//! pictures with "Add Picture..." (`control = "picture"`).

use ds_settings::schema::Page;
use serde::{Deserialize, Serialize};

/// The desktop's picture.
#[derive(
    Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, ds_settings::SettingsSchema,
)]
#[serde(default)]
#[settings(file = "sill/settings.toml", domain = "wallpaper", page = Page::declared("wallpaper"))]
pub struct WallpaperSettings {
    /// `wallpaper.picture`: a picture file's path. Empty is no picture, so the desktop shows its
    /// plain paper colour.
    #[settings(
        label = "Picture",
        help = "Empty shows the plain desktop colour.",
        section = "Desktop",
        control = "picture"
    )]
    pub picture: String,
}
