//! `hot_corners.*`: what each screen corner does when the pointer rests in it (design/20 §1.16;
//! macOS "Desktop & Dock" > Hot Corners, hence `Page::Dock`). design/13 has no dwell or action
//! table yet. All Advanced.

use crate::file::ShellFile;
use crate::section::Section;
use ds_core::word::Word;
use ds_settings::schema::Page;
use ds_settings::{Ms, Px};
use serde::{Deserialize, Serialize};

/// What a corner does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, Word)]
#[serde(rename_all = "snake_case")]
#[word(case = snake)]
pub enum CornerAction {
    /// Nothing, and no surface in that corner.
    #[default]
    None,
    Launcher,
    NotificationCenter,
    ControlCenter,
    /// Minimise every window of the workspace; the corner again brings them back.
    ShowDesktop,
    /// Lock the screen.
    Lock,
    /// COSMIC's workspace overview (`cosmic-workspaces`, COSMIC's own `WorkspaceOverview`).
    Workspaces,
    /// The corner's `*_command`, run by `/bin/sh -c`.
    Command,
}

/// The four corners.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ds_settings::SettingsSchema)]
#[serde(default)]
#[settings(file = "sill/settings.toml", domain = "hot_corners", page = Page::Dock)]
pub struct HotCornersSettings {
    #[settings(
        label = "Top left",
        help = "What the top-left corner does.",
        section = "Hot Corners",
        advanced
    )]
    pub top_left: CornerAction,
    #[settings(
        label = "Top right",
        help = "What the top-right corner does.",
        section = "Hot Corners",
        advanced
    )]
    pub top_right: CornerAction,
    #[settings(
        label = "Bottom left",
        help = "What the bottom-left corner does.",
        section = "Hot Corners",
        advanced
    )]
    pub bottom_left: CornerAction,
    #[settings(
        label = "Bottom right",
        help = "What the bottom-right corner does.",
        section = "Hot Corners",
        advanced
    )]
    pub bottom_right: CornerAction,
    #[settings(
        label = "Top-left command",
        help = "The command the top-left corner runs when it is set to Command.",
        section = "Hot Corners",
        advanced
    )]
    pub top_left_command: String,
    #[settings(
        label = "Top-right command",
        help = "The command the top-right corner runs when it is set to Command.",
        section = "Hot Corners",
        advanced
    )]
    pub top_right_command: String,
    #[settings(
        label = "Bottom-left command",
        help = "The command the bottom-left corner runs when it is set to Command.",
        section = "Hot Corners",
        advanced
    )]
    pub bottom_left_command: String,
    #[settings(
        label = "Bottom-right command",
        help = "The command the bottom-right corner runs when it is set to Command.",
        section = "Hot Corners",
        advanced
    )]
    pub bottom_right_command: String,
    #[settings(
        label = "Corner delay",
        help = "How long the pointer rests in a corner before it acts.",
        section = "Hot Corners",
        range = "0..=2000",
        unit = "ms",
        advanced
    )]
    pub dwell_ms: Ms,
    #[settings(
        label = "Corner re-arm time",
        help = "After acting, a corner acts again only once the pointer has left it and this long has passed.",
        section = "Hot Corners",
        range = "0..=5000",
        unit = "ms",
        advanced
    )]
    pub rearm_ms: Ms,
    #[settings(
        label = "Corner size",
        help = "The side of the invisible square in each active corner.",
        section = "Hot Corners",
        range = "1..=8",
        unit = "px",
        advanced
    )]
    pub size_px: Px,
}

impl Default for HotCornersSettings {
    fn default() -> Self {
        HotCornersSettings {
            top_left: CornerAction::None,
            top_right: CornerAction::None,
            bottom_left: CornerAction::None,
            bottom_right: CornerAction::None,
            top_left_command: String::new(),
            top_right_command: String::new(),
            bottom_left_command: String::new(),
            bottom_right_command: String::new(),
            dwell_ms: Ms(150),
            rearm_ms: Ms(500),
            size_px: Px(2),
        }
    }
}

impl Section for HotCornersSettings {
    const KEY: &'static str = "hot_corners";

    fn of(file: &ShellFile) -> &Self {
        &file.hot_corners
    }
}
