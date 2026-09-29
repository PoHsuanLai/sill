//! One module per surface (design/20 §1), each a component. The banners and the notification
//! center share one notification card; the control center is a bar popup.

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
pub mod notification_center;
pub mod osd;
pub mod polkit_prompt;
pub mod power_menu;
pub mod screenshot_thumb;
pub mod switcher;
pub mod user_picture;
pub mod wallpaper;

pub(crate) mod naming;
