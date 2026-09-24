//! One module per surface (design/20 §1). The four of milestones M1-M4 are components; the
//! switcher and notifications carry only the pure machines design/13 specifies, ahead of their
//! milestones.

pub mod bar;
pub mod dock;
pub mod launcher;
pub mod notifications;
pub mod switcher;
pub mod wallpaper;

pub(crate) mod naming;
