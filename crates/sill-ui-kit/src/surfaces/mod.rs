//! One module per surface (design/20 §1), each a component. The banners and the notification
//! center share one notification card; the control center is a bar popup.

pub mod bar;
pub mod control_center;
pub mod dock;
pub mod launcher;
