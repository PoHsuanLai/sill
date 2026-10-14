//! Which compositor the services' own Wayland connections (toplevels and workspaces, the
//! clipboard, idle) talk to.

use std::os::unix::net::UnixStream;
use std::path::PathBuf;

use wayland_client::{ConnectError, Connection};

/// How a service's Wayland connection is made.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub enum WaylandRoute {
    /// `$WAYLAND_DISPLAY` (or `$WAYLAND_SOCKET`), as every client does: the daemon's compositor.
    #[default]
    Environment,
    /// A socket by path: a live test's nested compositor, or one that is not there.
    Socket(PathBuf),
}

/// Why no connection was made.
#[derive(Debug, thiserror::Error)]
pub enum WaylandUnreached {
    #[error("no Wayland socket: {0}")]
    Socket(std::io::Error),
    #[error("no Wayland connection: {0}")]
    Connect(#[from] ConnectError),
}

impl WaylandRoute {
    /// A new connection by this route.
    pub fn connect(&self) -> Result<Connection, WaylandUnreached> {
        Ok(match self {
            WaylandRoute::Environment => Connection::connect_to_env()?,
            WaylandRoute::Socket(path) => Connection::from_socket(
                UnixStream::connect(path).map_err(WaylandUnreached::Socket)?,
            )?,
        })
    }
}
