//! Which surfaces exist on which output, and the pure reconciliation from the host's output
//! list to spawn and close actions.

#![allow(unused_variables, dead_code)] // Freeze stubs

use shell_host::{OutputId, OutputInfo, SurfaceId};

/// One output's surfaces (each `None` until spawned).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct OutputSurfaces {
    pub bar: Option<SurfaceId>,
    pub wallpaper: Option<SurfaceId>,
    pub dock: Option<SurfaceId>,
    pub launcher: Option<SurfaceId>,
    pub catcher: Option<SurfaceId>,
}

/// Every output's surfaces.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Outputs {
    pub outputs: Vec<(OutputInfo, OutputSurfaces)>,
}

/// What reconciling asks the daemon to do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OutputAction {
    SpawnAll(OutputInfo),
    /// Close every surface of a removed output.
    CloseAll(OutputId),
    /// Size or scale changed: re-apply sizes, re-prepare the wallpaper.
    Resize(OutputInfo),
}

/// The actions that turn `current` into one set of surfaces per output in `now`.
pub fn reconcile(current: &Outputs, now: &[OutputInfo]) -> Vec<OutputAction> {
    todo!("M1 daemon")
}
