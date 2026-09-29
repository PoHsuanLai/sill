//! Everything a service reaches outside the process, chosen once at the edge: the daemon builds
//! [`Env::from_process`], a test builds its own. No service reads an environment variable or
//! other ambient state to decide where to connect.

mod buses;

pub use buses::{Bus, BusKind, BusRoute, Buses};

/// The seams every service is started with.
#[derive(Debug, Clone)]
pub struct Env {
    pub buses: Buses,
}

impl Env {
    /// The process's own: its buses.
    pub fn from_process() -> Env {
        Env {
            buses: Buses::from_process(),
        }
    }
}
