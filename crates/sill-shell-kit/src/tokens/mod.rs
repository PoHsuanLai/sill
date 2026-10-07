//! The shell's metric tokens: the control center's sizes on the ladder, the dock, OSD,
//! notifications and the widget grid. Each family is a `Token` listed in `sill_shell_kit::KIT`, so the
//! stylesheet and the linter read them from there. The type scale, the menus' and bar's sizes and
//! the widgets' paints are `ds-style`'s, because generic components read them.

pub mod control_center;
pub mod dock;
pub mod glow;
pub mod notifications;
pub mod osd;
pub mod widgets;

#[cfg(test)]
mod scale_tests;
