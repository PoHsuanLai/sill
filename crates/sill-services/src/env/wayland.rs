//! Which compositor the services' own Wayland connections (toplevels and workspaces, the
//! clipboard, idle) talk to.

use std::hash::{Hash, Hasher};
use std::os::fd::{AsRawFd, OwnedFd};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::sync::Arc;

use wayland_client::{ConnectError, Connection};

/// How a service's Wayland connection is made.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub enum WaylandRoute {
    /// `$WAYLAND_DISPLAY` (or `$WAYLAND_SOCKET`), as every client does: the daemon's compositor.
    #[default]
    Environment,
    /// A socket by path: a live test's nested compositor, or one that is not there.
    Socket(PathBuf),
    /// A connected socket the compositor handed the shell (`org.quire.Compositor1.Connect`):
    /// the shell's own standing, where `$WAYLAND_DISPLAY` is any app's.
    Handed(HandedSocket),
}

/// A socket the compositor handed over. Each connection made by it is a duplicate of the one
/// descriptor, which identifies it while the route holds it open.
#[derive(Debug, Clone)]
pub struct HandedSocket(Arc<OwnedFd>);

impl HandedSocket {
    /// Hold `fd`, a connected Wayland socket.
    pub fn new(fd: OwnedFd) -> Self {
        Self(Arc::new(fd))
    }
}

impl PartialEq for HandedSocket {
    fn eq(&self, other: &Self) -> bool {
        self.0.as_raw_fd() == other.0.as_raw_fd()
    }
}

impl Eq for HandedSocket {}

impl Hash for HandedSocket {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.as_raw_fd().hash(state);
    }
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
            WaylandRoute::Handed(HandedSocket(fd)) => Connection::from_socket(UnixStream::from(
                fd.try_clone().map_err(WaylandUnreached::Socket)?,
            ))?,
        })
    }
}
