//! `dock.*` (design/22 §3.5, §4.1; design/10 §10.6). The Dock page shows the eleven keys §5
//! lists; the geometry and timing keys are Advanced. Pinned apps are state in `dock.json`, not
//! here.

use ds_settings::schema::Page;
use ds_settings::{Ms, Px};
use serde::{Deserialize, Serialize};

pub use crate::dock_words::{
    ActiveClick, AutoHide, Bounce, DockFloor, DockMenuOrder, DockModifierClicks, DockPosition,
    Indicators, LaunchAnim, Magnification, ProgressStyle, TrashTile,
};

/// The dock's settings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ds_settings::SettingsSchema)]
#[serde(default)]
#[settings(file = "sill/settings.toml", domain = "dock", page = Page::Dock)]
pub struct DockSettings {
    #[settings(
        label = "Magnification",
        help = "Icons grow as the pointer nears them.",
        section = "Magnification"
    )]
    pub magnification: Magnification,
    #[settings(
        label = "Size",
        help = "The size of a dock icon at rest.",
        section = "Size",
        range = "32..=80",
        unit = "px"
    )]
    pub tile_size_px: Px,
    #[settings(
        label = "Magnified size",
        help = "The size of the icon under the pointer.",
        section = "Magnification",
        range = "32..=128",
        unit = "px",
        advanced
    )]
    pub magnified_size_px: Px,
    #[settings(
        label = "Zoom reach",
        help = "How far from the pointer icons still grow.",
        section = "Magnification",
        range = "24..=256",
        unit = "px",
        advanced
    )]
    pub influence_radius_px: Px,
    #[settings(
        label = "Icon gap",
        help = "Space between dock icons.",
        section = "Size",
        range = "0..=16",
        unit = "px",
        advanced
    )]
    pub tile_gap_px: Px,
    #[settings(
        label = "Progress style",
        help = "A ring on the icon, or a bar under it.",
        section = "Indicators",
        advanced
    )]
    pub progress_style: ProgressStyle,
    #[settings(
        label = "Automatically hide the dock",
        help = "Hide the dock until the pointer reaches the screen's bottom edge.",
        section = "Hiding"
    )]
    pub autohide: AutoHide,
    #[settings(
        label = "Show delay",
        help = "How long the pointer rests at the edge before the dock appears.",
        section = "Hiding",
        range = "0..=1000",
        unit = "ms"
    )]
    pub autohide_delay_ms: Ms,
    #[settings(
        label = "Slide duration",
        help = "How long the dock takes to slide in or out.",
        section = "Hiding",
        range = "0..=1500",
        unit = "ms"
    )]
    pub autohide_slide_ms: Ms,
    #[settings(
        label = "Edge trigger",
        help = "The strip at the screen's edge that reveals a hidden dock.",
        section = "Hiding",
        range = "1..=16",
        unit = "px",
        advanced
    )]
    pub autohide_trigger_strip_px: Px,
    #[settings(
        label = "Position",
        help = "Which screen edge the dock sits on.",
        section = "Position"
    )]
    pub position: DockPosition,
    #[settings(
        label = "Show indicators for open apps",
        help = "A dot under each app that has a window.",
        section = "Indicators"
    )]
    pub indicators: Indicators,
    #[settings(
        label = "Bounce for attention",
        help = "Apps that need you bounce their icon.",
        section = "Animation"
    )]
    pub bounce: Bounce,
    #[settings(
        label = "Animate opening apps",
        help = "An icon bounces while its app starts.",
        section = "Animation"
    )]
    pub launch_animation: LaunchAnim,
    #[settings(
        label = "Clicking the active app",
        help = "Cycle through its windows, or do nothing.",
        section = "Clicking"
    )]
    pub click_active_app: ActiveClick,
    #[settings(
        label = "Show Trash",
        help = "A Trash tile at the end of the dock.",
        section = "Items"
    )]
    pub trash: TrashTile,
    #[settings(
        label = "Pill radius",
        help = "The corner radius of the dock's background.",
        section = "Size",
        range = "0..=48",
        unit = "px",
        advanced
    )]
    pub pill_radius_px: Px,
    #[settings(
        label = "Edge margin",
        help = "How close to the screen's sides the dock may come.",
        section = "Size",
        range = "0..=64",
        unit = "px",
        advanced
    )]
    pub edge_clamp_px: Px,
    #[settings(
        label = "Smallest icon",
        help = "How small icons may shrink when the dock is full.",
        section = "Size",
        range = "16..=48",
        unit = "px",
        advanced
    )]
    pub overflow_min_tile_px: Px,
    #[settings(
        label = "Indicator dot",
        help = "The diameter of the open-app dot.",
        section = "Indicators",
        range = "2..=12",
        unit = "px",
        advanced
    )]
    pub running_dot_diameter_px: Px,
    #[settings(
        label = "Badge height",
        help = "The height of a count badge.",
        section = "Indicators",
        range = "12..=32",
        unit = "px",
        advanced
    )]
    pub badge_size_px: Px,
    #[settings(
        label = "Badge radius",
        help = "The corner radius of a count badge.",
        section = "Indicators",
        range = "0..=16",
        unit = "px",
        advanced
    )]
    pub badge_radius_px: Px,
    #[settings(
        label = "Progress ring",
        help = "The diameter of a progress ring.",
        section = "Indicators",
        range = "12..=48",
        unit = "px",
        advanced
    )]
    pub progress_ring_diameter_px: Px,
    #[settings(
        label = "Progress ring stroke",
        help = "The width of a progress ring's arc.",
        section = "Indicators",
        range = "1..=8",
        unit = "px",
        advanced
    )]
    pub progress_ring_stroke_px: Px,
    #[settings(
        label = "Separator height",
        help = "The height of the line before Trash.",
        section = "Items",
        range = "8..=80",
        unit = "px",
        advanced
    )]
    pub separator_height_px: Px,
    #[settings(
        label = "Grow time",
        help = "How long icons take to grow when the pointer enters the dock.",
        section = "Magnification",
        range = "0..=1000",
        unit = "ms",
        advanced
    )]
    pub magnify_enter_ms: Ms,
    #[settings(
        label = "Shrink time",
        help = "How long icons take to settle when the pointer leaves the dock.",
        section = "Magnification",
        range = "0..=1000",
        unit = "ms",
        advanced
    )]
    pub magnify_leave_ms: Ms,
    #[settings(
        label = "Label offset",
        help = "The gap between an icon and its name label.",
        section = "Labels",
        range = "0..=32",
        unit = "px",
        advanced
    )]
    pub hover_label_offset_px: Px,
    #[settings(
        label = "Label warm time",
        help = "How long after leaving the dock the next label shows at once.",
        section = "Labels",
        range = "0..=2000",
        unit = "ms",
        advanced
    )]
    pub hover_label_warm_ms: Ms,
    #[settings(
        label = "Bounce period",
        help = "One bounce, up and down.",
        section = "Animation",
        range = "100..=2000",
        unit = "ms",
        advanced
    )]
    pub bounce_period_ms: Ms,
    #[settings(
        label = "Launch bounce limit",
        help = "The longest an opening app bounces.",
        section = "Animation",
        range = "0..=60000",
        unit = "ms",
        advanced
    )]
    pub bounce_launch_cap_ms: Ms,
    #[settings(
        label = "Hold for menu",
        help = "How long a press must hold still to open an icon's menu.",
        section = "Clicking",
        range = "100..=2000",
        unit = "ms",
        advanced
    )]
    pub hold_to_menu_ms: Ms,
    #[settings(
        label = "Hold tolerance",
        help = "How far a held press may move and still open the menu.",
        section = "Clicking",
        range = "1..=64",
        unit = "px",
        advanced
    )]
    pub hold_to_menu_move_px: Px,
    #[settings(
        label = "Remove distance",
        help = "How far an icon must be dragged out of the dock to remove it.",
        section = "Items",
        range = "16..=256",
        unit = "px",
        advanced
    )]
    pub remove_threshold_px: Px,
    #[settings(
        label = "Spring-loading delay",
        help = "How long a dragged file rests on an app before the app opens.",
        section = "Items",
        range = "100..=2000",
        unit = "ms",
        advanced
    )]
    pub spring_load_ms: Ms,
    #[settings(
        label = "Minimize target delay",
        help = "How long after the dock changes its layout the minimize targets are updated.",
        section = "Items",
        range = "0..=1000",
        unit = "ms",
        advanced
    )]
    pub minimize_debounce_ms: Ms,
    #[settings(
        label = "Menu order",
        help = "Windows first, or Apple's order.",
        section = "Clicking",
        advanced
    )]
    pub context_menu_order: DockMenuOrder,
    #[settings(
        label = "Modifier clicks",
        help = "Ctrl-click shows in Files, Alt-click switches and hides.",
        section = "Clicking",
        advanced
    )]
    pub modifier_clicks: DockModifierClicks,
    /// `dock.pill_padding_px` (the macOS polish pass: 6 on every side of the tiles).
    #[settings(
        label = "Dock padding",
        help = "The space between the icons and the dock's edge.",
        section = "Size",
        range = "0..=24",
        unit = "px",
        advanced
    )]
    pub pill_padding_px: Px,
    /// `dock.running_dot_gap_px`.
    #[settings(
        label = "Indicator dot gap",
        help = "From an icon's bottom edge to the centre of its open-app dot.",
        section = "Indicators",
        range = "0..=12",
        unit = "px",
        advanced
    )]
    pub running_dot_gap_px: Px,
    /// `dock.floor`.
    #[settings(
        label = "Floor",
        help = "A soft light band along the dock's floor, under the icons.",
        section = "Size",
        advanced
    )]
    pub floor: DockFloor,
}

impl Default for DockSettings {
    fn default() -> Self {
        DockSettings {
            magnification: Magnification::On,
            tile_size_px: Px(48),
            magnified_size_px: Px(96),
            influence_radius_px: Px(96),
            tile_gap_px: Px(8),
            progress_style: ProgressStyle::Ring,
            autohide: AutoHide::Off,
            autohide_delay_ms: Ms(200),
            autohide_slide_ms: Ms(500),
            autohide_trigger_strip_px: Px(4),
            position: DockPosition::Bottom,
            indicators: Indicators::On,
            bounce: Bounce::On,
            launch_animation: LaunchAnim::On,
            click_active_app: ActiveClick::Cycle,
            trash: TrashTile::On,
            pill_radius_px: Px(22),
            edge_clamp_px: Px(8),
            overflow_min_tile_px: Px(24),
            running_dot_diameter_px: Px(4),
            badge_size_px: Px(18),
            badge_radius_px: Px(9),
            progress_ring_diameter_px: Px(20),
            progress_ring_stroke_px: Px(3),
            separator_height_px: Px(36),
            magnify_enter_ms: Ms(120),
            magnify_leave_ms: Ms(200),
            hover_label_offset_px: Px(7),
            hover_label_warm_ms: Ms(400),
            bounce_period_ms: Ms(500),
            bounce_launch_cap_ms: Ms(10_000),
            hold_to_menu_ms: Ms(600),
            hold_to_menu_move_px: Px(8),
            remove_threshold_px: Px(64),
            spring_load_ms: Ms(500),
            minimize_debounce_ms: Ms(100),
            context_menu_order: DockMenuOrder::WindowsFirst,
            modifier_clicks: DockModifierClicks::AppleMapping,
            pill_padding_px: Px(6),
            running_dot_gap_px: Px(3),
            floor: DockFloor::Off,
        }
    }
}
