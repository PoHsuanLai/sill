//! `spaces.*` (design/22 §3.14; design/21): the defaults a workspace without a stored
//! `SpaceLook` gets. The per-workspace looks are state in `quire/spaces.json`, not here. The
//! mail frame and wallpaper policies are on the Spaces page (§5); the rest are Advanced.
//! `spaces.material_tint_alpha` is an alias of `appearance.material_tint_alpha` (quire's file),
//! not a second key.

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

/// `spaces.overlay_tint`: whether notifications, OSD, power menu, lock and polkit take the tint.
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
pub enum OverlayTint {
    #[default]
    Off,
    On,
}

/// `spaces.mail_frame_policy` (design/21 §11 open decision 1).
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
pub enum MailSpacePolicy {
    Own,
    Workspace,
    #[default]
    OwnFallbackWorkspace,
}

/// `spaces.wallpaper_follows_space` (design/21 §8).
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
pub enum WallpaperPolicy {
    #[default]
    Independent,
    PerWorkspace,
}

/// `spaces.wallpaper_drawer`: who paints the desktop background. COSMIC's own `cosmic-bg` by
/// default until shell-host repaints a background layer after its first commit (sill FINDINGS
/// F171, G21); `Shell` spawns sill's own wallpaper surface on every output.
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
    #[default]
    Cosmic,
    Shell,
}

/// `spaces.dock_look_source` (design/21 §11 open decision 5).
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
pub enum DockLookSource {
    #[default]
    OwnOutput,
    FocusedWindow,
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
        label = "Tint overlays",
        help = "Whether notifications, the OSD and system prompts take the Space's colour.",
        section = "Defaults",
        advanced
    )]
    pub overlay_tint: OverlayTint,
    #[settings(
        label = "Mail's frame",
        help = "Mail's own Space, the workspace's, or its own unless it has none.",
        section = "Apps"
    )]
    pub mail_frame_policy: MailSpacePolicy,
    #[settings(
        label = "Wallpaper",
        help = "One wallpaper for every Space, or one per workspace.",
        section = "Desktop"
    )]
    pub wallpaper_follows_space: WallpaperPolicy,
    #[settings(
        label = "Wallpaper drawn by",
        help = "COSMIC's own background service, or the shell's wallpaper, which cross-fades with light and dark.",
        section = "Desktop",
        advanced
    )]
    pub wallpaper_drawer: WallpaperDrawer,
    #[settings(
        label = "Dock colour source",
        help = "The dock's own screen's Space, or the focused window's.",
        section = "Desktop",
        advanced
    )]
    pub dock_look_source: DockLookSource,
    #[settings(
        label = "Look lookup",
        help = "Find a workspace's look by its id, then its position; or by position only.",
        section = "Defaults",
        advanced
    )]
    pub lookup_order: SpaceLookLookup,
    /// Keys this build does not know, kept for the next write.
    #[serde(flatten)]
    #[settings(skip)]
    pub extra: toml::Table,
}

impl Default for SpacesSettings {
    fn default() -> Self {
        SpacesSettings {
            default_grain: Count(40),
            default_card_accent: DefaultCardAccent::Postmark,
            overlay_tint: OverlayTint::Off,
            mail_frame_policy: MailSpacePolicy::OwnFallbackWorkspace,
            wallpaper_follows_space: WallpaperPolicy::Independent,
            wallpaper_drawer: WallpaperDrawer::Cosmic,
            dock_look_source: DockLookSource::OwnOutput,
            lookup_order: SpaceLookLookup::ByIdThenIndex,
            extra: toml::Table::new(),
        }
    }
}
