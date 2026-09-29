//! The two D-Bus buses the services talk to, each reached by a [`BusRoute`] chosen once when the
//! process starts (or by a test), never by a service reading the environment itself.

use zbus::Connection;

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

    /// A new connection to this bus; `Ok(None)` when the route is [`BusRoute::Absent`].
    pub async fn connect(&self) -> zbus::Result<Option<Connection>> {
        let connection = match (&self.route, self.kind) {
            (BusRoute::Absent, _) => return Ok(None),
            (BusRoute::Usual, BusKind::System) => Connection::system().await?,
            (BusRoute::Usual, BusKind::Session) => Connection::session().await?,
            (BusRoute::Address(address), _) => {
                zbus::connection::Builder::address(address.as_str())?
                    .build()
                    .await?
            }
        };
        Ok(Some(connection))
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
    use super::{BusRoute, Buses};

    #[tokio::test]
    async fn an_absent_route_connects_to_nothing() {
        let buses = Buses::absent();
        for bus in [&buses.system, &buses.session] {
            assert!(matches!(bus.connect().await, Ok(None)), "{bus:?}");
        }
    }

    #[tokio::test]
    async fn an_address_nobody_listens_on_is_an_error() {
        let buses = Buses::new(
            BusRoute::Address("unix:path=/nonexistent/sill-env-test".to_owned()),
            BusRoute::Absent,
        );
        assert!(buses.system.connect().await.is_err());
    }
}
