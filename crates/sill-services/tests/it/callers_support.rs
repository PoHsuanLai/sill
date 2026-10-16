//! The callers of the shell's bus objects, as a test names them: the peers of each connection a
//! service serves on are kept, so a stand-in can be made a service (or a program, or nobody)
//! whenever the test needs, by the unique name its connection has. Nothing reads `/proc` or asks
//! the unit manager.

use std::fmt;
use std::sync::{Arc, Mutex, PoisonError};

use peephole_bus::peephole::fake::{introduced_program, introduced_service, introduced_unverified};
use peephole_bus::peephole_core::{AppKey, ExePath, Pin, Proof, Standing, Table, WhyNot};
use peephole_bus::{BusPeers, ManagerCheck, Tokio};
use sill_services::callers::{CallerSource, Peers};
use zbus::Connection;

/// What a stand-in can be made.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Role {
    /// The portal backend.
    Portal,
    /// accountd.
    Accounts,
    /// inferd.
    Inference,
    /// syncd.
    Sync,
    /// The intent router.
    Router,
    /// Another service of the desktop, which none of the shell's objects answers.
    Settings,
}

impl Role {
    fn standing(self) -> Standing {
        let (app, role) = match self {
            Role::Portal => ("org.quire.Portal", "portal"),
            Role::Accounts => ("org.quire.Accounts", "accounts"),
            Role::Inference => ("org.quire.Inference", "inference"),
            Role::Sync => ("org.quire.Sync", "sync"),
            Role::Router => ("org.quire.Intents", "router"),
            Role::Settings => ("org.quire.Settings", "settings"),
        };
        introduced_service(app, role, Proof::Full, Pin::AtConnect)
            .unwrap_or_else(|| panic!("a valid standing")) // test-only
    }
}

/// What a stand-in is, to the shell.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Who {
    /// One of the desktop's services.
    Service(Role),
    /// A program the system owns, which is no service of the desktop.
    Program,
    /// Nobody the system can name: an image the person can write.
    Nobody,
}

/// The peers of every connection handed out, and who each stand-in is.
#[derive(Default)]
pub(crate) struct Introduced {
    peers: Mutex<Vec<Arc<Peers>>>,
    standings: Mutex<Vec<(String, Standing)>>,
}

impl fmt::Debug for Introduced {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Introduced").finish_non_exhaustive()
    }
}

impl Introduced {
    pub(crate) fn new() -> Arc<Introduced> {
        Arc::new(Introduced::default())
    }

    /// `who` is the service `role`, to every service that asks, now and later.
    pub(crate) fn service(&self, who: &Connection, role: Role) {
        self.introduce(who, role.standing());
    }

    /// `who` is a program the system owns (the image `path`), not a service.
    pub(crate) fn program(&self, who: &Connection, path: &str) {
        let standing = introduced_program(path).unwrap_or_else(|| panic!("a valid path")); // test-only
        self.introduce(who, standing);
    }

    /// The unique name `unique` is a program the system owns (the image `exe`) that the launcher
    /// started as the app `launched_as`.
    pub(crate) fn launched_as(&self, unique: &str, exe: &str, launched_as: &str) {
        let standing = Standing::Program {
            exe: ExePath::new(exe).unwrap_or_else(|e| panic!("{e}")), // test-only
            scope_app: Some(AppKey::new(launched_as).unwrap_or_else(|e| panic!("{e}"))), // test-only
        };
        self.introduce_name(unique, standing);
    }

    /// The unique name `name` stands as `standing`, for a stand-in that is another process.
    pub(crate) fn introduce_name(&self, name: &str, standing: Standing) {
        self.standings
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push((name.to_owned(), standing.clone()));
        for peers in self
            .peers
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .iter()
        {
            peers.introduce(name, standing.clone());
        }
    }

    /// `who` is `as_`, to every service that asks, now and later.
    pub(crate) fn make(&self, who: &Connection, as_: Who) {
        match as_ {
            Who::Service(role) => self.service(who, role),
            Who::Program => self.program(who, "/usr/bin/stranger"),
            Who::Nobody => self.introduce(who, introduced_unverified(WhyNot::ImageWritable)),
        }
    }

    fn introduce(&self, who: &Connection, standing: Standing) {
        let name = who
            .unique_name()
            .map(ToString::to_string)
            .unwrap_or_else(|| panic!("a connection with no name")); // test-only
        self.introduce_name(&name, standing);
    }
}

impl CallerSource for Introduced {
    fn peers_on(&self, connection: Connection) -> Arc<Peers> {
        let peers: Arc<Peers> = Arc::new(
            BusPeers::new(connection, Table::empty(), ManagerCheck::Unchecked).with_offload(Tokio),
        );
        for (name, standing) in self
            .standings
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .iter()
        {
            peers.introduce(name, standing.clone());
        }
        self.peers
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(Arc::clone(&peers));
        peers
    }
}
