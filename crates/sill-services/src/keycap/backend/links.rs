//! What the keycap service reads and drives, and where it connects.

use std::path::PathBuf;

use sill_model::apps::AppIndex;
use sill_model::cosmic_wl::CosmicWl;
use sill_model::notifications::{NotesState, NotificationsCommand};
use tokio::sync::watch;

use crate::control::Control;
use crate::env::{Bus, BusRoute, Env, UnitWait};

/// The other services keycap reads and drives.
#[derive(Debug, Clone)]
pub struct KeycapLinks {
    /// Which window has the focus.
    pub cosmic_wl: watch::Receiver<CosmicWl>,
    /// The desktop entries that tell which profile the focused window wants.
    pub apps: watch::Receiver<AppIndex>,
    /// Where the notices about keycap are posted and withdrawn.
    pub notifications: Control<NotificationsCommand>,
    /// What the notifications service holds, to find the notice already shown.
    pub notes: watch::Receiver<NotesState>,
}

/// Where the service connects.
#[derive(Debug, Clone)]
pub(crate) struct Reach {
    /// keycap's socket; `None` and the service never connects.
    pub(crate) socket: Option<PathBuf>,
    /// The system bus, where systemd answers about the unit; `None` when there is no such bus.
    pub(crate) system: Option<Bus>,
    /// When the unit is asked about.
    pub(crate) unit_wait: UnitWait,
    /// keycap's folder under the config directory.
    pub(crate) config_dir: PathBuf,
}

impl Reach {
    /// `env`'s keycap socket, system bus and config directory.
    pub(crate) fn of(env: &Env) -> Reach {
        let system = match env.buses.system.route() {
            BusRoute::Absent => None,
            BusRoute::Usual | BusRoute::Address(_) => Some(env.buses.system.clone()),
        };
        Reach {
            socket: env.keycap_socket.clone(),
            system,
            unit_wait: env.keycap_unit_wait,
            config_dir: env.dirs.keycap_config(),
        }
    }
}
