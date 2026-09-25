//! One module per surface (design/20 §1). The four of milestones M1-M4 and the OSD are
//! components, and M5's control center (a bar popup) and power menu are frozen components
//! (FINDINGS "M5 freeze"); the switcher and notifications carry only the pure machines design/13
//! specifies, ahead of their milestones.

pub mod bar;
pub mod control_center;
pub mod dock;
pub mod launcher;
pub mod notifications;
pub mod osd;
pub mod power_menu;
pub mod switcher;
pub mod wallpaper;

pub(crate) mod naming;
