//! Keeping a service alive: waiting between attempts with a growing delay, connecting to a bus
//! that is not up yet, and naming a panic that stopped a service. A service that cannot do its
//! work right now waits and tries again; it never ends for good because a peer was late.

use std::any::Any;
use std::time::{Duration, Instant};

use zbus::Connection;

use crate::env::Bus;
use crate::error::ServiceError;

/// The wait after the first failure: quick enough that a restart the person caused (a daemon
/// restarted by hand) is followed at once.
pub const FIRST_WAIT: Duration = Duration::from_millis(500);

/// The longest wait between attempts: a peer that is down for good is asked about twice a minute,
/// which costs nothing.
pub const LONGEST_WAIT: Duration = Duration::from_secs(30);

/// How long an attempt must have lasted to count as healthy, so that the next failure starts
/// again from [`FIRST_WAIT`] instead of continuing a long wait.
pub const HEALTHY_FOR: Duration = Duration::from_secs(60);

/// How long to wait before the next attempt: [`FIRST_WAIT`], doubling to [`LONGEST_WAIT`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Backoff {
    next: Duration,
}

impl Default for Backoff {
    fn default() -> Self {
        Backoff { next: FIRST_WAIT }
    }
}

impl Backoff {
    /// Waits that start at `first` instead of [`FIRST_WAIT`].
    pub fn starting_at(first: Duration) -> Backoff {
        Backoff { next: first }
    }

    /// The wait to sit out now; the one after it is twice as long, up to [`LONGEST_WAIT`].
    pub fn wait(&mut self) -> Duration {
        let now = self.next;
        self.next = (self.next * 2).min(LONGEST_WAIT);
        now
    }

    /// An attempt that ran for `lasted` ended: a healthy one starts the waits over.
    pub fn after_attempt(&mut self, lasted: Duration) {
        if lasted >= HEALTHY_FOR {
            *self = Backoff::default();
        }
    }
}

/// A connection to `bus`, waiting for the bus to come up when it is not there yet (a system bus
/// restarting, a session bus a late desktop starts). A bus that is absent by choice is the
/// [`ServiceError::NoBus`] it always was.
pub async fn connect_when_up(bus: &Bus) -> Result<Connection, ServiceError> {
    let mut backoff = Backoff::default();
    loop {
        let started = Instant::now();
        match bus.connect().await {
            Ok(Some(connection)) => return Ok(connection),
            Ok(None) => return Err(ServiceError::NoBus(bus.kind())),
            Err(error) => {
                let wait = backoff.wait();
                eprintln!(
                    "sill: the {:?} bus is not reachable ({error}); trying again in {wait:?}",
                    bus.kind()
                );
                backoff.after_attempt(started.elapsed());
                tokio::time::sleep(wait).await;
            }
        }
    }
}

/// The text of a panic payload (`panic!("..")` carries a `&str` or a `String`).
pub fn panic_text(payload: &(dyn Any + Send)) -> String {
    payload
        .downcast_ref::<&str>()
        .map(|text| (*text).to_owned())
        .or_else(|| payload.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "a panic without a message".to_owned())
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::{Backoff, FIRST_WAIT, HEALTHY_FOR, LONGEST_WAIT, panic_text};

    #[test]
    fn the_waits_double_up_to_the_longest() {
        let mut backoff = Backoff::default();
        let waits: Vec<Duration> = (0..9).map(|_| backoff.wait()).collect();
        assert_eq!(waits[0], FIRST_WAIT);
        assert_eq!(waits[1], FIRST_WAIT * 2);
        assert_eq!(waits[2], FIRST_WAIT * 4);
        assert_eq!(*waits.last().unwrap_or(&Duration::ZERO), LONGEST_WAIT);
        assert!(waits.windows(2).all(|pair| pair[0] <= pair[1]));
    }

    #[test]
    fn a_healthy_attempt_starts_the_waits_over_and_a_short_one_does_not() {
        let mut backoff = Backoff::default();
        for _ in 0..4 {
            backoff.wait();
        }
        backoff.after_attempt(HEALTHY_FOR - Duration::from_secs(1));
        assert_eq!(backoff.wait(), FIRST_WAIT * 16);
        backoff.after_attempt(HEALTHY_FOR);
        assert_eq!(backoff.wait(), FIRST_WAIT);
    }

    #[test]
    fn a_panic_payload_is_read_as_text() {
        let from_str: Box<dyn std::any::Any + Send> = Box::new("boom");
        let from_string: Box<dyn std::any::Any + Send> = Box::new(String::from("bang"));
        let other: Box<dyn std::any::Any + Send> = Box::new(7_u8);
        assert_eq!(panic_text(from_str.as_ref()), "boom");
        assert_eq!(panic_text(from_string.as_ref()), "bang");
        assert_eq!(panic_text(other.as_ref()), "a panic without a message");
    }
}
