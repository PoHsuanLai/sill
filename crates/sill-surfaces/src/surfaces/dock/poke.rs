//! Presses a harness gives the dock in place of real ones. shell-host injects primary clicks
//! only (G10), so nothing can press the secondary button on a dock under test; a harness that
//! provides [`DockPokes`] as a root context (`examples/dock_demo.rs`) feeds the machine the
//! same press a real pointer would. The daemon provides none, so its docks never read one.

use tokio::sync::watch;

use crate::surfaces::dock::machine::Button;

/// One synthetic press, surface-local logical pixels; `seq` tells two equal presses apart.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DockPoke {
    pub seq: u64,
    pub x: i32,
    pub y: i32,
    pub button: Button,
}

/// The harness's presses, the latest in the channel.
#[derive(Debug, Clone)]
pub struct DockPokes(pub watch::Receiver<Option<DockPoke>>);
