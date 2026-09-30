//! One module per surface (design/20 §1), each a component. The banners and the notification
//! center share one notification card; the control center is a bar popup.

pub mod banners;
pub mod bar;
pub mod control_center;
pub mod desktop_widgets;
pub mod dock;
pub mod edit_widgets;
pub mod launcher;
pub mod notification_center;
pub mod screenshot_thumb;
