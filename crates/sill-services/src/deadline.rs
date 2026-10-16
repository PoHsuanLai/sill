//! How long the shell waits on a peer. Every await on a program outside sill (an app, a system
//! daemon, the compositor) is bounded by one of these, so a peer that stops answering costs one
//! failed step and never wedges the loop that asked.

use std::future::Future;
use std::time::Duration;

use crate::error::ServiceError;

/// One method call on an app or a system service: long enough for a daemon the bus has to start
/// first, short enough that a hung one is given up on while the person still remembers asking.
pub const BUS_CALL: Duration = Duration::from_secs(8);

/// A whole launch-service command (activation, or a hand-over and then a fallback): more than
/// one [`BUS_CALL`], so a slow first step still leaves room for the fallback.
pub const LAUNCH_COMMAND: Duration = Duration::from_secs(20);

/// Reading NetworkManager's whole access point list or saved connections: one call per entry, so
/// a long list of entries all answering slowly still ends.
pub const NETWORK_REFRESH: Duration = Duration::from_secs(15);

/// A command that goes to a status-notifier app: it is the app's own menu or action, and it can
/// be as hung as the app is.
pub const TRAY_CALL: Duration = Duration::from_secs(5);

/// A request to the companion service (an ask, a card answer): the assistant may think, but the
/// shell does not wait on the connection for it.
pub const COMPANION_CALL: Duration = Duration::from_secs(10);

/// Await `work` for at most `limit`; `what` names it in the error.
pub async fn within<T>(
    limit: Duration,
    what: &'static str,
    work: impl Future<Output = T>,
) -> Result<T, ServiceError> {
    tokio::time::timeout(limit, work)
        .await
        .map_err(|_| ServiceError::TimedOut(what))
}

/// [`within`] for a bus call: a call that answered with an error is the bus error, one that did
/// not answer is [`ServiceError::TimedOut`].
pub async fn call<T>(
    limit: Duration,
    what: &'static str,
    work: impl Future<Output = zbus::Result<T>>,
) -> Result<T, ServiceError> {
    within(limit, what, work).await?.map_err(ServiceError::Bus)
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::{call, within};
    use crate::error::ServiceError;

    #[tokio::test]
    async fn a_peer_that_never_answers_is_given_up_on() {
        let waited = within(
            Duration::from_millis(20),
            "a peer",
            std::future::pending::<()>(),
        )
        .await;
        assert!(matches!(waited, Err(ServiceError::TimedOut("a peer"))));
    }

    #[tokio::test]
    async fn an_answer_in_time_comes_through() {
        let answered = within(Duration::from_secs(5), "a peer", async { 7 }).await;
        assert!(matches!(answered, Ok(7)));
    }

    #[tokio::test]
    async fn a_bus_error_stays_a_bus_error_and_silence_is_a_timeout() {
        let refused = call(Duration::from_secs(5), "a call", async {
            Err::<(), _>(zbus::Error::Unsupported)
        })
        .await;
        assert!(matches!(refused, Err(ServiceError::Bus(_))));
        let silent = call(
            Duration::from_millis(20),
            "a call",
            std::future::pending::<zbus::Result<()>>(),
        )
        .await;
        assert!(matches!(silent, Err(ServiceError::TimedOut("a call"))));
    }
}
