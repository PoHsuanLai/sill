//! One module per surface (design/20 §1). The four of milestones M1-M4 and the OSD are
//! components, M5's control center (a bar popup) and power menu likewise (FINDINGS "M5
//! freeze"), and M6's banners and notification center are frozen components sharing one
//! notification card (FINDINGS "M6 freeze"); M10's screenshot thumbnail, hot corners and
//! desktop widget layer are frozen components (FINDINGS "M10 freeze"); the switcher carries
//! only the pure machine design/13 specifies, ahead of its milestone.

pub mod banners;
pub mod bar;
pub mod control_center;
pub mod desktop_widgets;
pub mod dock;
pub mod hot_corners;
pub mod launcher;
pub mod notification_card;
pub mod notification_center;
pub mod osd;
pub mod power_menu;
pub mod screenshot_thumb;
pub mod switcher;
pub mod wallpaper;

pub(crate) mod naming;
