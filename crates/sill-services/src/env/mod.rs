//! Everything a service reaches outside the process, chosen once at the edge: the daemon builds
//! [`Env::from_process`], a test builds its own. No service reads an environment variable or
//! other ambient state to decide where to connect.

mod authenticator;
mod buses;
pub mod ddc;
mod dirs;
mod processes;
mod session_actions;
mod wall_clock;
mod wayland;

use std::path::Path;
use std::sync::Arc;

use crate::auth::PamHelper;
use crate::idle::IdleTimes;
use crate::polkit_agent;
use ddc::{DdcBus, FileBus, I2cBuses};

pub use authenticator::{Authenticator, FakeAuthenticator, FakeRule};
pub use buses::{Bus, BusKind, BusRoute, Buses};
pub use dirs::Dirs;
pub use processes::{NoProcesses, Pid, ProcScan, Processes};
pub use session_actions::{
    ActionReach, DryRunActions, DryRunOffers, LogindActions, SessionActions, dry_run_line,
};
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
    /// The DDC/CI buses external monitors answer on.
    pub ddc: Arc<dyn DdcBus>,
    /// Where idle's screen-off and suspend times come from.
    pub idle_times: IdleTimesSource,
    /// Where the polkit agent registers: polkitd's bus, polkit's helper, the session.
    pub polkit: polkit_agent::PolkitRoute,
    /// Where the session's actions go: logind, or a dry run's record.
    pub session_actions: Arc<dyn SessionActions>,
    /// What checks the lock screen's password: PAM through `sill auth-check`, or a stand-in.
    pub authenticator: Arc<dyn Authenticator>,
    /// The processes a service looks for by name: a `/proc` scan, or none.
    pub processes: Arc<dyn Processes>,
}

/// Where idle's screen-off and suspend times come from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum IdleTimesSource {
    /// The settings (`idle.times` and its keys, in minutes).
    #[default]
    Settings,
    /// These, for both power sources, whatever the settings say (a dev script's seconds).
    Fixed(IdleTimes),
}

impl Env {
    /// The process's own: its buses, its compositor, its directories, the system clock, the
    /// kernel's i2c buses, polkitd on the system bus, logind's session actions and PAM through
    /// `sill auth-check`.
    pub fn from_process() -> Env {
        let buses = Buses::from_process();
        Env {
            polkit: polkit_agent::PolkitRoute::system(buses.system.clone()),
            buses,
            wayland: WaylandRoute::Environment,
            dirs: Dirs::from_process(),
            clock: Arc::new(SystemClock),
            ddc: Arc::new(I2cBuses::default()),
            idle_times: IdleTimesSource::Settings,
            session_actions: Arc::new(LogindActions),
            authenticator: Arc::new(PamHelper::this_exe()),
            processes: Arc::new(ProcScan),
        }
    }

    /// Nothing outside the process: no bus, a compositor socket in `scratch` that does not exist,
    /// every directory inside `scratch`, the system clock (read only), DDC monitors from a file
    /// that is not there (none), no polkit helper, session actions that are only recorded, and a
    /// password check that refuses everything, and no processes running. For tests that start services and must never reach
    /// the person's session or files. `scratch` must exist: the wallpaper service watches the
    /// nearest existing folder above COSMIC's record.
    pub fn isolated(scratch: &Path) -> Env {
        Env {
            buses: Buses::absent(),
            wayland: WaylandRoute::Socket(scratch.join("no-wayland-socket")),
            dirs: Dirs::under(scratch),
            clock: Arc::new(SystemClock),
            ddc: Arc::new(FileBus::new(scratch.join("no-ddc-monitors.json"))),
            idle_times: IdleTimesSource::Settings,
            polkit: polkit_agent::PolkitRoute {
                bus: Bus::system(BusRoute::Absent),
                helper: polkit_agent::HelperChoice::Use(polkit_agent::HelperRoute::Missing),
                session: polkit_agent::SessionChoice::Id("sill-test".to_owned()),
            },
            session_actions: Arc::new(DryRunActions::new(DryRunOffers::Everything, None)),
            authenticator: Arc::new(FakeAuthenticator {
                rule: FakeRule::RefusesAll,
                delay: std::time::Duration::ZERO,
            }),
            processes: Arc::new(NoProcesses),
        }
    }
}
