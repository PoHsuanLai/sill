//! The two D-Bus buses the services talk to, each reached by a [`BusRoute`] chosen once when the
//! process starts (or by a test), never by a service reading the environment itself.

use std::time::Instant;

use zbus::Connection;

use crate::supervise::Backoff;

/// Which of the usual buses a [`Bus`] stands for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BusKind {
    System,
    Session,
}

/// How a bus is reached.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub enum BusRoute {
    /// The process's own: `DBUS_SYSTEM_BUS_ADDRESS` or `DBUS_SESSION_BUS_ADDRESS`, else the
    /// standard socket.
    #[default]
    Usual,
    /// A bus by address: a private `dbus-daemon` with stand-ins (tests, dev scripts).
    Address(String),
    /// No bus at all: whatever needs it stays at its absent default.
    Absent,
}

/// One bus a service talks to: which one, and how it is reached.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Bus {
    kind: BusKind,
    route: BusRoute,
}

impl Bus {
    /// The system bus, reached by `route`.
    pub fn system(route: BusRoute) -> Bus {
        Bus {
            kind: BusKind::System,
            route,
        }
    }

    /// The session bus, reached by `route`.
    pub fn session(route: BusRoute) -> Bus {
        Bus {
            kind: BusKind::Session,
            route,
        }
    }

    /// Which bus this is.
    pub fn kind(&self) -> BusKind {
        self.kind
    }

    /// How it is reached.
    pub fn route(&self) -> &BusRoute {
        &self.route
    }

    /// A new connection to this bus, once: no waiting for a bus that is not up yet. For a caller
    /// that has its own way to ask again; every service that runs for the session uses
    /// [`Bus::connect_waiting`].
    pub async fn try_connect(&self) -> Result<Connection, ConnectError> {
        self.open().await.map_err(|error| match error {
            OpenError::Absent => ConnectError::NoBus(self.kind),
            OpenError::Refused(error) => ConnectError::Refused(error.to_string()),
        })
    }

    /// A new connection to this bus, waiting for the bus to come up when it is not there yet (a
    /// system bus restarting, a session bus a late desktop starts). A bus that is absent by
    /// choice is [`ConnectError::NoBus`] at once; `Refused` never comes out of this one.
    pub async fn connect_waiting(&self) -> Result<Connection, ConnectError> {
        let mut backoff = Backoff::default();
        loop {
            let started = Instant::now();
            match self.open().await {
                Ok(connection) => return Ok(connection),
                Err(OpenError::Absent) => return Err(ConnectError::NoBus(self.kind)),
                Err(OpenError::Refused(error)) => {
                    let wait = backoff.wait();
                    eprintln!(
                        "sill: the {:?} bus is not reachable ({error}); trying again in {wait:?}",
                        self.kind
                    );
                    backoff.after_attempt(started.elapsed());
                    tokio::time::sleep(wait).await;
                }
            }
        }
    }

    /// One attempt at the route.
    async fn open(&self) -> Result<Connection, OpenError> {
        let connection = match (&self.route, self.kind) {
            (BusRoute::Absent, _) => return Err(OpenError::Absent),
            (BusRoute::Usual, BusKind::System) => Connection::system().await,
            (BusRoute::Usual, BusKind::Session) => Connection::session().await,
            (BusRoute::Address(address), _) => {
                zbus::connection::Builder::address(address.as_str())?
                    .build()
                    .await
            }
        };
        connection.map_err(OpenError::Refused)
    }
}

/// Why a bus could not be reached.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ConnectError {
    /// The bus is absent by choice ([`BusRoute::Absent`]).
    #[error("no {0:?} bus")]
    NoBus(BusKind),
    /// The bus is configured and did not answer; the reason in words.
    #[error("{0}")]
    Refused(String),
}

/// One attempt at a route that did not give a connection.
enum OpenError {
    Absent,
    Refused(zbus::Error),
}

impl From<zbus::Error> for OpenError {
    fn from(error: zbus::Error) -> Self {
        OpenError::Refused(error)
    }
}

/// The system and the session bus.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Buses {
    pub system: Bus,
    pub session: Bus,
}

impl Buses {
    /// The system bus by `system`, the session bus by `session`.
    pub fn new(system: BusRoute, session: BusRoute) -> Buses {
        Buses {
            system: Bus::system(system),
            session: Bus::session(session),
        }
    }

    /// The process's own two buses.
    pub fn from_process() -> Buses {
        Buses::new(BusRoute::Usual, BusRoute::Usual)
    }

    /// Neither bus: every service that needs one stays at its absent default.
    pub fn absent() -> Buses {
        Buses::new(BusRoute::Absent, BusRoute::Absent)
    }
}

#[cfg(test)]
mod tests {
    use super::{BusRoute, Buses, ConnectError};

    #[tokio::test]
    async fn an_absent_route_connects_to_nothing() {
        let buses = Buses::absent();
        for bus in [&buses.system, &buses.session] {
            assert_eq!(
                bus.try_connect().await.err(),
                Some(ConnectError::NoBus(bus.kind())),
                "{bus:?}"
            );
        }
    }

    #[tokio::test]
    async fn an_address_nobody_listens_on_is_an_error() {
        let buses = Buses::new(
            BusRoute::Address("unix:path=/nonexistent/sill-env-test".to_owned()),
            BusRoute::Absent,
        );
        assert!(matches!(
            buses.system.try_connect().await,
            Err(ConnectError::Refused(_))
        ));
    }
}
