//! Who the stand-in voiced is to the shell: the voice service, or a program that only owns its
//! name, as a test says. Nothing reads `/proc` or asks the unit manager.

use std::fmt;
use std::sync::{Arc, Mutex, PoisonError};

use peephole_bus::peephole::fake::{introduced_program, introduced_service};
use peephole_bus::peephole_core::{Pin, Proof, Standing, Table};
use peephole_bus::{BusPeers, ManagerCheck, Offload};
use sill_services::callers::{CallerSource, Peers};
use zbus::Connection;

/// What the program behind the stand-in's connection is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VoicedIs {
    /// voiced: the program the caller table gives the voice role.
    TheVoiceService,
    /// Some other program that has taken the name.
    OnlyOwnsTheName,
}

impl VoicedIs {
    fn standing(self) -> Standing {
        match self {
            VoicedIs::TheVoiceService => {
                introduced_service("org.quire.Voice", "voice", Proof::Full, Pin::AtConnect)
            }
            VoicedIs::OnlyOwnsTheName => introduced_program("/usr/bin/stranger"),
        }
        .unwrap_or_else(|| panic!("a valid standing")) // test-only
    }
}

/// The peers of the shell's connection, in which the stand-in's connection is `VoicedIs`.
pub struct VoicedCallers {
    voiced: String,
    peers: Mutex<Vec<Arc<Peers>>>,
    is: Mutex<VoicedIs>,
}

impl fmt::Debug for VoicedCallers {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("VoicedCallers").finish_non_exhaustive()
    }
}

impl VoicedCallers {
    pub fn new(voiced: &Connection, is: VoicedIs) -> Arc<VoicedCallers> {
        let voiced = voiced
            .unique_name()
            .map(ToString::to_string)
            .unwrap_or_else(|| panic!("a connection with no name")); // test-only
        Arc::new(VoicedCallers {
            voiced,
            peers: Mutex::new(Vec::new()),
            is: Mutex::new(is),
        })
    }

    /// From now on the stand-in is `is`, to every service that asks.
    pub fn become_(&self, is: VoicedIs) {
        *self.is.lock().unwrap_or_else(PoisonError::into_inner) = is;
        for peers in self
            .peers
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .iter()
        {
            peers.introduce(&self.voiced, is.standing());
        }
    }
}

impl CallerSource for VoicedCallers {
    fn peers_on(&self, connection: Connection) -> Arc<Peers> {
        let peers: Arc<Peers> = Arc::new(
            BusPeers::new(connection, Table::empty(), ManagerCheck::Unchecked)
                .with_offload(Offload::Tokio),
        );
        let is = *self.is.lock().unwrap_or_else(PoisonError::into_inner);
        peers.introduce(&self.voiced, is.standing());
        self.peers
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(Arc::clone(&peers));
        peers
    }
}
