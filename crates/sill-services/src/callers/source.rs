//! Where a service gets the means to tell its callers apart: the table of the desktop's own
//! programs, the unit manager's word, and the system's `/proc`, all chosen once at the edge.

use std::fmt::Debug;
use std::sync::Arc;

use peephole_bus::peephole::{RealProc, TableFiles};
use peephole_bus::peephole_core::{Proof, Requirement, Table, roles};
use peephole_bus::{BusPeers, ManagerCheck, ManagerLink, Offload};
use zbus::Connection;

/// How callers on a bus are told apart: their process, held by a pidfd, judged against the caller
/// table. The probe runs on tokio's blocking pool.
pub type Peers = BusPeers<RealProc, ManagerLink<RealProc>>;

/// What gives a service the [`Peers`] of the connection it serves on. The shell's is
/// [`DesktopCallers`]; a test brings its own, which keeps the peers it hands out so that it can
/// make its stand-ins whoever it needs, whenever it needs.
pub trait CallerSource: Debug + Send + Sync {
    /// The means to tell the callers on `connection` apart.
    fn peers_on(&self, connection: Connection) -> Arc<Peers>;
}

/// Callers judged against the desktop's table, with the unit manager's answers believed only when
/// the manager's own image is the system's (role `manager`, proved by its image).
#[derive(Debug, Clone)]
pub struct DesktopCallers {
    pub(super) table: Table,
}

impl DesktopCallers {
    /// Callers judged against `table`.
    pub fn new(table: Table) -> DesktopCallers {
        DesktopCallers { table }
    }

    /// Callers judged against the table the desktop ships (`/etc/quire/desktop-callers.toml` and
    /// the files beside it). When it cannot be loaded (missing, or writable by someone other than
    /// root) the table is empty: nobody is a service, and every call is refused.
    pub fn from_system() -> DesktopCallers {
        let table = TableFiles::desktop()
            .load(&RealProc::system())
            .unwrap_or_else(|error| {
                eprintln!(
                    "sill: callers: the table of the desktop's own programs could not be loaded ({error}), so every call to the shell's bus objects is refused"
                );
                Table::empty()
            });
        DesktopCallers::new(table)
    }
}

impl CallerSource for DesktopCallers {
    fn peers_on(&self, connection: Connection) -> Arc<Peers> {
        let manager = ManagerCheck::Verified(Requirement::new(roles::MANAGER, Proof::Exe));
        Arc::new(
            BusPeers::new(connection, self.table.clone(), manager).with_offload(Offload::Tokio),
        )
    }
}
