//! Everything a service reaches outside the process, chosen once at the edge: the daemon builds
//! [`Env::from_process`], a test builds its own. No service reads an environment variable or
//! other ambient state to decide where to connect.

use std::path::Path;
use std::sync::Arc;

mod buses;
mod dirs;
#[cfg(test)]
pub(crate) mod scratch;
mod wall_clock;
mod wayland;

pub use buses::{Bus, BusKind, BusRoute, Buses};
pub use dirs::Dirs;
pub use wall_clock::{FixedClock, SystemClock, WallClock};
pub use wayland::{WaylandRoute, WaylandUnreached};

/// The seams every service is started with.
#[derive(Debug, Clone)]
pub struct Env {
    pub buses: Buses,
    /// The compositor the services' own Wayland connections talk to.
    pub wayland: WaylandRoute,
    /// The person's directories.
    pub dirs: Dirs,
    /// The wall clock the services read.
    pub clock: Arc<dyn WallClock>,
}

impl Env {
    /// The process's own: its buses, its compositor, its directories and the system clock.
    pub fn from_process() -> Env {
        Env {
            buses: Buses::from_process(),
            wayland: WaylandRoute::Environment,
            dirs: Dirs::from_process(),
            clock: Arc::new(SystemClock),
        }
    }

    /// Nothing outside the process: no bus, a compositor socket in `scratch` that does not
    /// exist, every directory inside `scratch`, and the system clock (read only). For tests that start services and must never
    /// reach the person's session or files. `scratch` must exist: the wallpaper service watches
    /// the nearest existing folder above COSMIC's record.
    pub fn isolated(scratch: &Path) -> Env {
        Env {
            buses: Buses::absent(),
            wayland: WaylandRoute::Socket(scratch.join("no-wayland-socket")),
            dirs: Dirs::under(scratch),
            clock: Arc::new(SystemClock),
        }
    }
}
