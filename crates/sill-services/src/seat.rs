//! The connection the shell holds at intentd. intentd names a caller's role from the well-known
//! names its connection owns, so everything sill says to intentd and the companion as the
//! launcher (a turn, a search, an undo) must go out on the connection that owns `org.quire.Shell`.
//! The intents service claims the names and publishes that connection here; the companion
//! service follows it.

use tokio::sync::watch;
use zbus::Connection;

/// The intents service's half: it publishes the connection once the names are claimed and
/// withdraws it when the seat is given up.
#[derive(Debug, Clone)]
pub struct SeatPublisher(watch::Sender<Option<Connection>>);

/// A follower's half.
#[derive(Debug, Clone)]
pub struct SeatView(watch::Receiver<Option<Connection>>);

/// A seat nobody holds yet.
pub fn seat() -> (SeatPublisher, SeatView) {
    let (tx, rx) = watch::channel(None);
    (SeatPublisher(tx), SeatView(rx))
}

impl SeatPublisher {
    /// The shell's connection, with its names claimed.
    pub fn publish(&self, connection: Connection) {
        self.0.send_replace(Some(connection));
    }

    /// The seat was given up.
    pub fn withdraw(&self) {
        self.0.send_replace(None);
    }
}

impl SeatView {
    /// The connection now, if the seat is held.
    pub fn current(&self) -> Option<Connection> {
        self.0.borrow().clone()
    }

    /// Resolves when the seat is taken or given up; an error once the intents service is gone.
    pub async fn changed(&mut self) -> Result<(), watch::error::RecvError> {
        self.0.changed().await
    }
}
