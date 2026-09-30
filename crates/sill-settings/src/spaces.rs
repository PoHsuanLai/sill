//! `spaces.*` (design/22 §3.14; design/21): the defaults a workspace without a stored
//! `SpaceLook` gets. The per-workspace looks are state in `quire/spaces.json`, not here. All
//! Advanced.

use crate::file::ShellFile;
use crate::section::Section;
use ds_core::word::Word;
use ds_settings::Count;
use ds_settings::schema::Page;
use serde::{Deserialize, Serialize};

/// `spaces.default_card_accent`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, Word)]
#[serde(rename_all = "snake_case")]
#[word(case = snake)]
pub enum DefaultCardAccent {
    #[default]
    Chosen,
    SpaceHue,
}

/// `spaces.wallpaper_drawer`: who paints the desktop background. `Shell` (default) spawns sill's
/// own wallpaper surface on every output; `Cosmic` leaves it to COSMIC's own `cosmic-bg`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, Word)]
#[serde(rename_all = "snake_case")]
#[word(case = snake)]
pub enum WallpaperDrawer {
    Cosmic,
    #[default]
    Shell,
}

/// Space defaults.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ds_settings::SettingsSchema)]
#[serde(default)]
#[settings(file = "sill/settings.toml", domain = "spaces", page = Page::Spaces)]
pub struct SpacesSettings {
    #[settings(
        label = "Default grain",
        help = "The grain of a Space that has no look of its own (presets 1 and 2 keep theirs).",
        section = "Defaults",
        range = "0..=100",
        advanced
    )]
    pub default_grain: Count,
    #[settings(
        label = "Default accent",
        help = "The accent you picked, or the Space's own hue.",
        section = "Defaults",
        advanced
    )]
    pub default_card_accent: DefaultCardAccent,
    #[settings(
        label = "Wallpaper drawn by",
        help = "COSMIC's own background service, or the shell's wallpaper, which cross-fades with light and dark.",
        section = "Desktop",
        advanced
    )]
    pub wallpaper_drawer: WallpaperDrawer,
}

impl Default for SpacesSettings {
    fn default() -> Self {
        SpacesSettings {
            default_grain: Count(40),
            default_card_accent: DefaultCardAccent::Chosen,
            wallpaper_drawer: WallpaperDrawer::Shell,
        }
    }
}

impl Section for SpacesSettings {
    const KEY: &'static str = "spaces";

    fn of(file: &ShellFile) -> &Self {
        &file.spaces
    }
}
