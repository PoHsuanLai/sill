//! Everything a service reaches outside the process, chosen once at the edge: the daemon builds
//! [`Env::from_process`], a test builds its own. No service reads an environment variable or
//! other ambient state to decide where to connect.

use std::path::Path;

mod buses;
mod dirs;
#[cfg(test)]
pub(crate) mod scratch;
mod wayland;

pub use buses::{Bus, BusKind, BusRoute, Buses};
pub use dirs::Dirs;
pub use wayland::{WaylandRoute, WaylandUnreached};

/// The seams every service is started with.
#[derive(Debug, Clone)]
pub struct Env {
    pub buses: Buses,
    /// The compositor the services' own Wayland connections talk to.
    pub wayland: WaylandRoute,
    /// The person's directories.
    pub dirs: Dirs,
}

impl Env {
    /// The process's own: its buses, its compositor and its directories.
    pub fn from_process() -> Env {
        Env {
            buses: Buses::from_process(),
            wayland: WaylandRoute::Environment,
            dirs: Dirs::from_process(),
        }
    }

    /// Nothing outside the process: no bus, a compositor socket in `scratch` that does not
    /// exist, and every directory inside `scratch`. For tests that start services and must never
    /// reach the person's session or files. `scratch` must exist: the wallpaper service watches
    /// the nearest existing folder above COSMIC's record.
    pub fn isolated(scratch: &Path) -> Env {
        Env {
            buses: Buses::absent(),
            wayland: WaylandRoute::Socket(scratch.join("no-wayland-socket")),
            dirs: Dirs::under(scratch),
        }
    }
}
