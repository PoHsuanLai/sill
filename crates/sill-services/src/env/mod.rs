//! Everything a service reaches outside the process, chosen once at the edge: the daemon builds
//! [`Env::from_process`], a test builds its own. No service reads an environment variable or
//! other ambient state to decide where to connect.

mod authenticator;
mod bus_name;
mod buses;
mod children;
pub mod ddc;
mod dirs;
mod passwd;
mod release;
mod session_actions;
mod wall_clock;
mod wayland;

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use crate::auth::PamHelper;
use crate::callers::{CallerSource, DesktopCallers};
use crate::polkit_agent;
use ddc::{DdcBus, FileBus, I2cBuses};
use sill_model::auth::{RealName, UserName};
use sill_model::idle::IdleTimes;
use sill_model::keycap::{UNIT_GRACE, UNIT_RECHECK};

pub use authenticator::{Authenticator, FakeAuthenticator, FakeRule};
pub use bus_name::{NameState, name_state};
pub use buses::{Bus, BusKind, BusRoute, Buses};
pub use children::{Children, ProcessGroup, RealChildren, RecordedChildren, Start};
pub use dirs::Dirs;
pub use passwd::{FakePasswd, Passwd, Person, SystemPasswd, UserId};
pub use release::{SESSION_MODE_VAR, SessionMode};
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
    /// Who this process runs as, for the lock screen: the system's accounts database, or a
    /// stand-in that never reads it.
    pub passwd: Arc<dyn Passwd>,
    /// The children a service starts and does not wait for (the file opener, the trash): real
    /// processes, or a record of what would have started.
    pub children: Arc<dyn Children>,
    /// keycap's control socket; `None` means the keycap service never connects.
    pub keycap_socket: Option<PathBuf>,
    /// How long keycap's socket may be down before systemd is asked about the unit, and how often
    /// after that; the process's own is the model's, a test's is short.
    pub keycap_unit_wait: UnitWait,
    /// The `/proc` tree a process's cgroup is read from, which names the app that launched it
    /// (porter's caller identity): `/proc`, or a fixture tree in a test, or a folder with
    /// nothing in it, where no process is proven to be any app.
    pub proc_root: PathBuf,
    /// How the callers of the shell's bus objects and the owners of the apps' menu names are
    /// told apart: the desktop's table and the system's `/proc`, or a stand-in in a test.
    pub callers: Arc<dyn CallerSource>,
    /// `XDG_CURRENT_DESKTOP` as the process was started with it (colon-separated names): read
    /// once at the edge, so a library never reads the environment.
    pub current_desktop: String,
    /// Whether the session fell back to the safe compositor (`QUIRE_SESSION_MODE`), read once at
    /// the edge.
    pub session_mode: SessionMode,
    /// The folder of the program this process runs, where the release's other programs are;
    /// `None` when it is not known (and in a test).
    pub release_bin: Option<PathBuf>,
}

/// When the keycap service asks systemd about its unit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnitWait {
    /// How long the socket is down before the first question.
    pub grace: Duration,
    /// How long between questions after that.
    pub recheck: Duration,
}

impl Default for UnitWait {
    fn default() -> UnitWait {
        UnitWait {
            grace: UNIT_GRACE,
            recheck: UNIT_RECHECK,
        }
    }
}

/// Where idle's screen-off and suspend times come from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum IdleTimesSource {
    /// The settings (the `idle.*_min` keys, in minutes).
    #[default]
    Settings,
    /// These, for both power sources, whatever the settings say (a dev script's seconds).
    Fixed(IdleTimes),
}

impl Env {
    /// The process's own: its buses, its compositor, its directories, the system clock, the
    /// kernel's i2c buses, polkitd on the system bus, logind's session actions, PAM through
    /// `sill auth-check`, the system's accounts database and keycap's socket under `/run/keycap`.
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
            passwd: Arc::new(SystemPasswd {
                fallback_login: std::env::var("USER").unwrap_or_default(),
            }),
            children: Arc::new(RealChildren),
            keycap_socket: Some(PathBuf::from(keycap_protocol::SOCKET_PATH)),
            keycap_unit_wait: UnitWait::default(),
            proc_root: PathBuf::from("/proc"),
            callers: Arc::new(DesktopCallers::from_system()),
            current_desktop: std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_default(),
            session_mode: SessionMode::from_process(),
            release_bin: release::running_bin_dir(),
        }
    }

    /// Nothing outside the process: no bus, a compositor socket in `scratch` that does not exist,
    /// every directory inside `scratch`, the system clock (read only), DDC monitors from a file
    /// that is not there (none), no polkit helper, session actions that are only recorded, and a
    /// password check that refuses everything, a made-up person for the account, no processes
    /// running, no child started (each is recorded), no keycap socket and callers judged against an empty table (every call to a
    /// shell object is refused until a test says who its stand-ins are). For tests that start
    /// services and must never reach the person's session or files. `scratch` must exist: the
    /// wallpaper service watches the nearest existing folder above the wallpaper's source file.
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
                helper: polkit_agent::HelperChoice::Use(
                    sill_model::polkit_agent::HelperRoute::Missing,
                ),
                session: polkit_agent::SessionChoice::Id("sill-test".to_owned()),
            },
            session_actions: Arc::new(DryRunActions::new(DryRunOffers::Everything, None)),
            authenticator: Arc::new(FakeAuthenticator {
                rule: FakeRule::RefusesAll,
                delay: std::time::Duration::ZERO,
            }),
            passwd: Arc::new(FakePasswd(Person {
                id: UserId(4242),
                user: UserName("sill-test".to_owned()),
                name: RealName("Test Person".to_owned()),
            })),
            children: RecordedChildren::shared(),
            keycap_socket: None,
            keycap_unit_wait: UnitWait::default(),
            proc_root: scratch.join("no-proc"),
            // An empty table: nobody is a service, so nothing is let in until a test says who.
            callers: Arc::new(DesktopCallers::new(
                peephole_bus::peephole_core::Table::empty(),
            )),
            current_desktop: String::new(),
            session_mode: SessionMode::Normal,
            release_bin: None,
        }
    }
}
