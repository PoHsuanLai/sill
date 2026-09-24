//! The dock's pinned apps, `~/.config/sill/dock.json` (design/10 §10.6: "pinned items stay in
//! `~/.config/sill/dock.json`"; state, not a setting, design/22 §1 rule 3).

#![allow(unused_variables, dead_code)] // Freeze stubs

mod model;

pub use model::{DockPins, PinAt, PinEvent, reduce_pins};

use std::path::Path;

use tokio::runtime::Handle;
use tokio::sync::watch;

use crate::control::Control;
use crate::error::ServiceError;

/// The file's name inside `$XDG_CONFIG_HOME/sill`.
pub const FILE_NAME: &str = "dock.json";

/// Load the pins, then apply every [`PinEvent`] sent to the control, writing the file
/// atomically after each.
pub fn spawn(rt: &Handle) -> (watch::Receiver<DockPins>, Control<PinEvent>) {
    todo!("M2 launch (dock_config): load, reduce_pins per command, save")
}

/// The pins in `dir`; a missing or damaged file is no pins.
pub fn load(dir: &Path) -> DockPins {
    todo!("M2 launch (dock_config): read FILE_NAME, DockPins::from_json")
}

/// Write `pins` to `dir` by temp file and rename.
pub fn save(dir: &Path, pins: &DockPins) -> Result<(), ServiceError> {
    todo!("M2 launch (dock_config)")
}
