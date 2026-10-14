//! The connection the compositor hands the shell: `org.quire.Compositor1.Connect("shell")`
//! returns a connected Wayland socket on which the privileged globals (`quire_shell_v1`) are
//! offered. They are never on the session's socket.
//!
//! [`Handout`] is the seam: [`ZbusHandout`] calls the compositor over the session bus,
//! [`FakeHandout`] yields a socket made by the test.

use std::os::fd::OwnedFd;
use std::sync::Mutex;

use crate::env::Bus;

/// Which privileged connection to ask for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionRole {
    /// The desktop shell's: `quire_shell_v1` and the shell role.
    Shell,
}

impl ConnectionRole {
    /// The role's name on the bus.
    pub const fn word(self) -> &'static str {
        match self {
            ConnectionRole::Shell => "shell",
        }
    }
}

/// Why no connection was handed over.
#[derive(Debug, thiserror::Error)]
pub enum HandoutError {
    /// The bus call failed (no compositor owns the name, or it refused this caller).
    #[error("the compositor's connection service: {0}")]
    Bus(#[from] zbus::Error),
    /// There is nothing to hand over (a fake that has run out).
    #[error("no connection to hand over")]
    Nothing,
}

/// Something that hands the shell a Wayland connection.
pub trait Handout {
    /// A new connected socket for `role`.
    fn connect(&self, role: ConnectionRole) -> impl Future<Output = Result<OwnedFd, HandoutError>>;
}

/// `org.quire.Compositor1`, which casement serves on the session bus.
#[zbus::proxy(
    interface = "org.quire.Compositor1",
    default_service = "org.quire.Compositor1",
    default_path = "/org/quire/Compositor1"
)]
trait Compositor1 {
    /// A connected Wayland socket for `role` (`"shell"` or `"agent"`).
    fn connect(&self, role: &str) -> zbus::Result<zbus::zvariant::OwnedFd>;
}

/// The compositor, asked over a bus connection.
#[derive(Debug, Clone)]
pub struct ZbusHandout {
    connection: zbus::Connection,
}

impl ZbusHandout {
    /// Ask over `connection` (the session bus).
    pub fn new(connection: zbus::Connection) -> Self {
        Self { connection }
    }
}

impl Handout for ZbusHandout {
    fn connect(&self, role: ConnectionRole) -> impl Future<Output = Result<OwnedFd, HandoutError>> {
        let connection = self.connection.clone();
        async move {
            let proxy = Compositor1Proxy::new(&connection).await?;
            Ok(proxy.connect(role.word()).await?.into())
        }
    }
}

/// A handout of one socket made by the test, or of none.
#[derive(Debug, Default)]
pub struct FakeHandout {
    socket: Mutex<Option<OwnedFd>>,
}

impl FakeHandout {
    /// Hands over `socket` once.
    pub fn handing(socket: OwnedFd) -> Self {
        Self {
            socket: Mutex::new(Some(socket)),
        }
    }

    /// Hands over nothing: the compositor is not there.
    pub fn empty() -> Self {
        Self::default()
    }
}

impl Handout for FakeHandout {
    fn connect(
        &self,
        _role: ConnectionRole,
    ) -> impl Future<Output = Result<OwnedFd, HandoutError>> {
        let socket = self.socket.lock().ok().and_then(|mut held| held.take());
        std::future::ready(socket.ok_or(HandoutError::Nothing))
    }
}

/// The shell's connection from the compositor on `bus`, or `None` when there is no session bus
/// or no compositor of ours owns the name (the session is then another desktop's or a test, and the
/// shell connects through the environment). Any other failure goes to stderr once.
pub async fn shell_connection(bus: &Bus) -> Option<OwnedFd> {
    let connection = match bus.connect().await {
        Ok(Some(connection)) => connection,
        Ok(None) => return None,
        Err(error) => {
            eprintln!("sill: desktop: the session bus: {error}");
            return None;
        }
    };
    match ZbusHandout::new(connection)
        .connect(ConnectionRole::Shell)
        .await
    {
        Ok(fd) => Some(fd),
        Err(HandoutError::Bus(zbus::Error::MethodError(name, ..))) if is_absent(name.as_str()) => {
            None
        }
        Err(error) => {
            eprintln!("sill: desktop: {error}; connecting through the environment");
            None
        }
    }
}

/// D-Bus's two ways of saying nobody owns the name.
fn is_absent(error_name: &str) -> bool {
    matches!(
        error_name,
        "org.freedesktop.DBus.Error.ServiceUnknown" | "org.freedesktop.DBus.Error.NameHasNoOwner"
    )
}
