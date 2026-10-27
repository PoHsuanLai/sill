//! `dock.*` (design/22 §3.5, §4.1; design/10 §10.6). The Dock page shows the eleven keys §5
//! lists; the geometry and timing keys are Advanced. Pinned apps are placements in `layout.toml`, not
//! here.

use ds_settings::schema::Page;
use ds_settings::{Ms, Px};
use serde::{Deserialize, Serialize};

pub use crate::dock_words::{
    ActiveClick, AutoHide, DockFloor, DockMenuOrder, DockModifierClicks, Indicators, Magnification,
    TrashTile,
};

/// The dock's settings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ds_settings::SettingsSchema)]
#[serde(default)]
#[settings(file = "sill/settings.toml", domain = "dock", page = Page::Dock)]
pub struct DockSettings {
    #[settings(
        label = "Size",
        help = "How big icons are at rest.",
        section = "Dock",
        range = "32..=80",
        unit = "px"
    )]
    pub tile_size_px: Px,
    #[settings(
        label = "Magnification",
        help = "Icons grow under the pointer.",
        section = "Dock"
    )]
    pub magnification: Magnification,
    #[settings(
        label = "Magnified size",
        help = "How big the icon under the pointer gets.",
        section = "Dock",
        range = "32..=72",
        unit = "px",
        advanced
    )]
    pub magnified_size_px: Px,
    #[settings(
        label = "Zoom reach",
        help = "How far from the pointer icons still grow.",
        section = "Dock",
        range = "24..=256",
        unit = "px",
        advanced
    )]
    pub influence_radius_px: Px,
    #[settings(
        label = "Icon gap",
        help = "Space between icons.",
        section = "Dock",
        range = "0..=16",
        unit = "px",
        advanced
    )]
    pub tile_gap_px: Px,
    #[settings(
        label = "Automatically hide the Dock",
        help = "Shows the Dock when the pointer reaches the bottom.",
        section = "Hiding"
    )]
    pub autohide: AutoHide,
    #[settings(
        label = "Show delay",
        help = "How long the pointer waits before the Dock appears.",
        section = "Hiding",
        range = "0..=1000",
        unit = "ms"
    )]
    pub autohide_delay_ms: Ms,
    #[settings(
        label = "Slide duration",
        help = "How long the Dock takes to slide.",
        section = "Hiding",
        range = "0..=1500",
        unit = "ms"
    )]
    pub autohide_slide_ms: Ms,
    #[settings(
        label = "Edge trigger",
        help = "How thick the strip is that reveals a hidden Dock.",
        section = "Hiding",
        range = "1..=16",
        unit = "px",
        advanced
    )]
    pub autohide_trigger_strip_px: Px,
    #[settings(
        label = "Show indicators for open apps",
        help = "A dot under each app with a window.",
        section = "Behaviour"
    )]
    pub indicators: Indicators,
    #[settings(
        label = "Clicking the active app",
        help = "Cycles through its windows, or does nothing.",
        section = "Behaviour"
    )]
    pub click_active_app: ActiveClick,
    #[settings(
        label = "Show Trash",
        help = "Adds a Trash tile at the end.",
        section = "Behaviour"
    )]
    pub trash: TrashTile,
    #[settings(
        label = "Corner radius",
        help = "How round the Dock's background is.",
        section = "Dock",
        range = "0..=48",
        unit = "px",
        advanced
    )]
    pub pill_radius_px: Px,
    #[settings(
        label = "Edge margin",
        help = "How close the Dock gets to the screen's sides.",
        section = "Dock",
        range = "0..=64",
        unit = "px",
        advanced
    )]
    pub edge_clamp_px: Px,
    #[settings(
        label = "Indicator dot",
        help = "How big the open-app dot is.",
        section = "Behaviour",
        range = "2..=12",
        unit = "px",
        advanced
    )]
    pub running_dot_diameter_px: Px,
    #[settings(
        label = "Separator height",
        help = "The height of the line before Trash.",
        section = "Behaviour",
        range = "8..=80",
        unit = "px",
        advanced
    )]
    pub separator_height_px: Px,
    #[settings(
        label = "Grow time",
        help = "How long icons take to grow.",
        section = "Dock",
        range = "0..=1000",
        unit = "ms",
        advanced
    )]
    pub magnify_enter_ms: Ms,
    #[settings(
        label = "Shrink time",
        help = "How long icons take to shrink back.",
        section = "Dock",
        range = "0..=1000",
        unit = "ms",
        advanced
    )]
    pub magnify_leave_ms: Ms,
    #[settings(
        label = "Quick labels",
        help = "How long after leaving the Dock labels show at once.",
        section = "Behaviour",
        range = "0..=2000",
        unit = "ms",
        advanced
    )]
    pub hover_label_warm_ms: Ms,
    #[settings(
        label = "Hold for menu",
        help = "How long to press before an icon's menu opens.",
        section = "Behaviour",
        range = "100..=2000",
        unit = "ms",
        advanced
    )]
    pub hold_to_menu_ms: Ms,
    #[settings(
        label = "Hold tolerance",
        help = "How far a press can move and still open the menu.",
        section = "Behaviour",
        range = "1..=64",
        unit = "px",
        advanced
    )]
    pub hold_to_menu_move_px: Px,
    #[settings(
        label = "Remove distance",
        help = "How far to drag an icon out to remove it.",
        section = "Behaviour",
        range = "16..=256",
        unit = "px",
        advanced
    )]
    pub remove_threshold_px: Px,
    #[settings(
        label = "Spring-loading delay",
        help = "How long a dragged file waits on an app before it opens.",
        section = "Behaviour",
        range = "100..=2000",
        unit = "ms",
        advanced
    )]
    pub spring_load_ms: Ms,
    #[settings(
        label = "Menu order",
        help = "The usual order, or open windows first.",
        section = "Behaviour",
        advanced
    )]
    pub context_menu_order: DockMenuOrder,
    #[settings(
        label = "Modifier clicks",
        help = "Ctrl-click shows in Files, Alt-click switches and hides.",
        section = "Behaviour",
        advanced
    )]
    pub modifier_clicks: DockModifierClicks,
    /// `dock.pill_padding_px` (the macOS polish pass: 6 on every side of the tiles).
    #[settings(
        label = "Dock padding",
        help = "Space between the icons and the Dock's edge.",
        section = "Dock",
        range = "0..=24",
        unit = "px",
        advanced
    )]
    pub pill_padding_px: Px,
    /// `dock.running_dot_gap_px`.
    #[settings(
        label = "Indicator dot gap",
        help = "Space from an icon's bottom to its dot.",
        section = "Behaviour",
        range = "0..=12",
        unit = "px",
        advanced
    )]
    pub running_dot_gap_px: Px,
    /// `dock.floor`.
    #[settings(
        label = "Floor",
        help = "A soft light band under the icons.",
        section = "Dock",
        advanced
    )]
    pub floor: DockFloor,
}

impl Default for DockSettings {
    fn default() -> Self {
        DockSettings {
            magnification: Magnification::On,
            tile_size_px: Px(48),
            magnified_size_px: Px(72),
            influence_radius_px: Px(72),
            tile_gap_px: Px(8),
            autohide: AutoHide::Off,
            autohide_delay_ms: Ms(200),
            autohide_slide_ms: Ms(500),
            autohide_trigger_strip_px: Px(4),
            indicators: Indicators::On,
            click_active_app: ActiveClick::Cycle,
            trash: TrashTile::On,
            pill_radius_px: Px(22),
            edge_clamp_px: Px(8),
            running_dot_diameter_px: Px(4),
            separator_height_px: Px(36),
            magnify_enter_ms: Ms(120),
            magnify_leave_ms: Ms(200),
            hover_label_warm_ms: Ms(400),
            hold_to_menu_ms: Ms(600),
            hold_to_menu_move_px: Px(8),
            remove_threshold_px: Px(64),
            spring_load_ms: Ms(500),
            context_menu_order: DockMenuOrder::AppleOrder,
            modifier_clicks: DockModifierClicks::AppleMapping,
            pill_padding_px: Px(6),
            running_dot_gap_px: Px(3),
            floor: DockFloor::Off,
        }
    }
}
