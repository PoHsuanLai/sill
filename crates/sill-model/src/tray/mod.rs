//! The status tray: SNI items and their dbusmenu rows, as the bar draws them.

mod command;
mod label;
mod model;
mod step;
#[cfg(test)]
mod tests;

pub use command::{ScrollAxis, ScrollDelta, TrayCommand, TrayEvent};
pub use label::plain_label;
pub use model::{
    Check, Pixmap, Tray, TrayIcon, TrayItem, TrayItemId, TrayMenuEntry, TrayMenuIcon, TrayMenuKind,
    TrayStatus,
};
pub use step::step;
