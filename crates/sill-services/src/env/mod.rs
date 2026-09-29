//! Everything a service reaches outside the process, chosen once at the edge: the daemon builds
//! [`Env::from_process`], a test builds its own. No service reads an environment variable or
//! other ambient state to decide where to connect.

use std::path::Path;

mod buses;
mod wayland;

pub use buses::{Bus, BusKind, BusRoute, Buses};
pub use wayland::{WaylandRoute, WaylandUnreached};

/// The seams every service is started with.
#[derive(Debug, Clone)]
pub struct Env {
    pub buses: Buses,
    /// The compositor the services' own Wayland connections talk to.
    pub wayland: WaylandRoute,
}

impl Env {
    /// The process's own: its buses and its compositor.
    pub fn from_process() -> Env {
        Env {
            buses: Buses::from_process(),
            wayland: WaylandRoute::Environment,
        }
    }

    /// Nothing outside the process: no bus, and a compositor socket in `scratch` that does not
    /// exist. For tests that start services and must never reach the person's session.
    pub fn isolated(scratch: &Path) -> Env {
        Env {
            buses: Buses::absent(),
            wayland: WaylandRoute::Socket(scratch.join("no-wayland-socket")),
        }
    }
}
