//! `spaces.*` (design/22 §3.14; design/21): the defaults a workspace without a stored
//! `SpaceLook` gets. The per-workspace looks are state in `quire/spaces.json`, not here. All
//! Advanced.

use ds_settings::Count;
use ds_settings::schema::Page;
use serde::{Deserialize, Serialize};

/// `spaces.default_card_accent`.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    Default,
    Serialize,
    Deserialize,
    ds_settings::SettingsSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum DefaultCardAccent {
    #[default]
    Postmark,
    SpaceHue,
}

/// `spaces.wallpaper_drawer`: who paints the desktop background. `Shell` (default) spawns sill's
/// own wallpaper surface on every output; `Cosmic` leaves it to COSMIC's own `cosmic-bg`.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    Default,
    Serialize,
    Deserialize,
    ds_settings::SettingsSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum WallpaperDrawer {
    Cosmic,
    #[default]
    Shell,
}

/// `spaces.lookup_order`. design/22 lists one variant and names `ByIndexOnly` as the future
/// second; a settings enum needs two, so it is here now (FINDINGS.md).
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    Default,
    Serialize,
    Deserialize,
    ds_settings::SettingsSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum SpaceLookLookup {
    /// `by_id[workspace id]`, else `by_index[position]`, else the default (design/21 §10).
    #[default]
    ByIdThenIndex,
    /// `by_index[position]` only, for a compositor whose ids are not stable.
    ByIndexOnly,
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
        help = "Postmark blue, or the Space's own hue.",
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
    #[settings(
        label = "Look lookup",
        help = "Find a workspace's look by its id, then its position; or by position only.",
        section = "Defaults",
        advanced
    )]
    pub lookup_order: SpaceLookLookup,
}

impl Default for SpacesSettings {
    fn default() -> Self {
        SpacesSettings {
            default_grain: Count(40),
            default_card_accent: DefaultCardAccent::Postmark,
            wallpaper_drawer: WallpaperDrawer::Shell,
            lookup_order: SpaceLookLookup::ByIdThenIndex,
        }
    }
}
