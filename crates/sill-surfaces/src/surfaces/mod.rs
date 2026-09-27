//! One module per surface (design/20 §1). The four of milestones M1-M4 and the OSD are
//! components, M5's control center (a bar popup) and power menu likewise (FINDINGS "M5
//! freeze"), and M6's banners and notification center are frozen components sharing one
//! notification card (FINDINGS "M6 freeze"); M10's screenshot thumbnail, hot corners and
//! desktop widget layer are frozen components (FINDINGS "M10 freeze"); M11's own lock screen,
//! polkit prompt and app switcher are frozen components (FINDINGS "M11 freeze").

pub mod banners;
pub mod bar;
pub mod control_center;
pub mod desktop_widgets;
pub mod dock;
pub mod edit_widgets;
pub mod hot_corners;
pub mod idle_dim;
pub mod launcher;
pub mod lock;
pub mod notification_card;
pub mod notification_center;
pub mod osd;
pub mod polkit_prompt;
pub mod power_menu;
pub mod screenshot_thumb;
pub mod switcher;
pub mod user_picture;
pub mod wallpaper;

pub(crate) mod naming;
