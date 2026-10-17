//! When a service that ended is started again. A service's task can panic or return while the shell
//! is still running, and without a restart its readers keep the last value for the rest of the
//! session. A restart waits (the growing [`Backoff`] of [`crate::supervise`]) and gives up after
//! [`RESTARTS_MAX`] tries in a row, so a service that cannot run is left alone and said so once.
//! A try that ran for [`HEALTHY_FOR`] before it ended does not count: the next ending starts the
//! count and the waits over.
//!
//! This file is the decision, pure over how the task ended; `service::spawn` carries it out.

use std::time::Duration;

use tokio::task::JoinError;

use crate::error::ServiceError;
use crate::supervise::{Backoff, FIRST_WAIT, HEALTHY_FOR, panic_text};

/// How many restarts in a row before a service is left stopped: with the waits doubling from
/// [`FIRST_WAIT`], about half a minute of trying.
pub const RESTARTS_MAX: u32 = 5;

/// A service that returns without an error sooner than this after it started has found nothing to
/// do here (no hardware, no bus to follow), and starting it again would find the same: it is
/// left stopped, without a log line. A clean end after longer than this is a service that was
/// working and stopped (a closed event stream), which a restart repairs.
pub const NOTHING_TO_DO_WITHIN: Duration = Duration::from_secs(10);

/// How a service's task ended.
#[derive(Debug)]
pub(crate) enum Ending {
    /// It returned without an error while somebody could still send it commands.
    Returned,
    /// It returned an error worth a restart, or panicked; the text says which.
    Failed(String),
    /// It cannot run here and never will: no bus by choice, a compositor without its protocol.
    Impossible(Option<String>),
    /// The runtime is shutting down.
    Cancelled,
}

/// How `ended` (a service task's result) is read.
pub(crate) fn ending_of(ended: Result<Result<(), ServiceError>, JoinError>) -> Ending {
    match ended {
        Ok(Ok(())) => Ending::Returned,
        Ok(Err(ServiceError::NoBus(_))) => Ending::Impossible(None),
        Ok(Err(error @ ServiceError::Unsupported(_))) => {
            Ending::Impossible(Some(error.to_string()))
        }
        Ok(Err(error)) => Ending::Failed(error.to_string()),
        Err(error) => match error.try_into_panic() {
            Ok(payload) => Ending::Failed(format!("panicked: {}", panic_text(payload.as_ref()))),
            Err(_) => Ending::Cancelled,
        },
    }
}

/// What to do about an ending.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Next {
    /// Start it again after this wait; the line says why, for the journal.
    Restart { wait: Duration, line: String },
    /// Leave it stopped; the line, when there is one, says why, for the journal.
    Stop { line: Option<String> },
}

/// The tries a service has used up.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Tries {
    first: Duration,
    backoff: Backoff,
    failed: u32,
}

impl Default for Tries {
    fn default() -> Self {
        Tries::starting_at(FIRST_WAIT)
    }
}

impl Tries {
    /// Tries whose waits start at `first`.
    pub(crate) fn starting_at(first: Duration) -> Tries {
        Tries {
            first,
            backoff: Backoff::starting_at(first),
            failed: 0,
        }
    }

    /// What follows `service` ending as `ending` after `lasted`.
    pub(crate) fn after(&mut self, service: &str, ending: &Ending, lasted: Duration) -> Next {
        let why = match ending {
            Ending::Cancelled => return Next::Stop { line: None },
            Ending::Impossible(why) => {
                return Next::Stop {
                    line: why
                        .as_ref()
                        .map(|why| format!("sill: {service}: {why}; it keeps its last value")),
                };
            }
            Ending::Returned if lasted < NOTHING_TO_DO_WITHIN => {
                return Next::Stop { line: None };
            }
            Ending::Returned => "ended".to_owned(),
            Ending::Failed(why) => why.clone(),
        };
        if lasted >= HEALTHY_FOR {
            *self = Tries::starting_at(self.first);
        }
        self.failed += 1;
        if self.failed > RESTARTS_MAX {
            return Next::Stop {
                line: Some(format!(
                    "sill: {service}: {why}; giving up after {RESTARTS_MAX} restarts, it keeps its last value"
                )),
            };
        }
        let wait = self.backoff.wait();
        Next::Restart {
            line: format!(
                "sill: {service}: {why}; starting it again in {wait:?} (restart {} of {RESTARTS_MAX})",
                self.failed
            ),
            wait,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::{Ending, NOTHING_TO_DO_WITHIN, Next, RESTARTS_MAX, Tries, ending_of};
    use crate::env::BusKind;
    use crate::error::ServiceError;
    use crate::supervise::{FIRST_WAIT, HEALTHY_FOR};

    async fn panicked() -> Ending {
        ending_of(tokio::spawn(async { panic_now() }).await)
    }

    fn panic_now() -> Result<(), ServiceError> {
        panic!("the thing broke")
    }

    #[tokio::test]
    async fn how_a_task_ended_decides_whether_it_is_started_again() {
        let ending_text = |ending: &Ending| format!("{ending:?}");
        let cases = [
            ("a clean end", ending_of(Ok(Ok(()))), "Returned"),
            (
                "a panic",
                panicked().await,
                "Failed(\"panicked: the thing broke\")",
            ),
            (
                "an error",
                ending_of(Ok(Err(ServiceError::Gone))),
                "Failed(\"the service has stopped\")",
            ),
            (
                "no bus by choice",
                ending_of(Ok(Err(ServiceError::NoBus(BusKind::System)))),
                "Impossible(None)",
            ),
            (
                "a missing protocol",
                ending_of(Ok(Err(ServiceError::Unsupported("wl_thing")))),
                "Impossible(Some(\"the compositor does not offer wl_thing\"))",
            ),
        ];
        for (name, ending, want) in cases {
            assert_eq!(ending_text(&ending), want, "{name}");
        }
    }

    #[test]
    fn a_service_is_started_again_with_growing_waits_and_then_left_alone() {
        let mut tries = Tries::default();
        let ending = Ending::Failed("panicked: boom".to_owned());
        let quick = Duration::from_secs(1);
        for n in 0..RESTARTS_MAX {
            let Next::Restart { wait, line } = tries.after("svc", &ending, quick) else {
                panic!("restart {n} was refused");
            };
            assert_eq!(wait, FIRST_WAIT * 2_u32.pow(n), "restart {n}");
            assert!(line.starts_with("sill: svc: panicked: boom; starting it again"));
        }
        let Next::Stop { line } = tries.after("svc", &ending, quick) else {
            panic!("it was started a sixth time");
        };
        assert!(line.is_some_and(|line| line.contains("giving up after 5 restarts")));
    }

    #[test]
    fn a_run_that_lasted_does_not_use_up_the_tries() {
        let mut tries = Tries::default();
        let ending = Ending::Returned;
        for _ in 0..(RESTARTS_MAX * 3) {
            let Next::Restart { wait, .. } = tries.after("svc", &ending, HEALTHY_FOR) else {
                panic!("a service that ran healthily was given up on");
            };
            assert_eq!(wait, FIRST_WAIT, "the waits start over");
        }
    }

    #[test]
    fn a_service_that_returns_cleanly_at_once_has_nothing_to_do_and_one_that_ran_is_restarted() {
        let mut tries = Tries::default();
        assert_eq!(
            tries.after("svc", &Ending::Returned, Duration::from_secs(1)),
            Next::Stop { line: None },
            "returned at once"
        );
        let after_a_run = tries.after("svc", &Ending::Returned, NOTHING_TO_DO_WITHIN);
        assert!(
            matches!(after_a_run, Next::Restart { wait, .. } if wait == FIRST_WAIT),
            "{after_a_run:?}"
        );
    }

    #[test]
    fn what_cannot_run_here_or_was_cancelled_is_not_started_again() {
        let mut tries = Tries::default();
        let quick = Duration::ZERO;
        assert_eq!(
            tries.after("svc", &Ending::Cancelled, quick),
            Next::Stop { line: None }
        );
        assert_eq!(
            tries.after("svc", &Ending::Impossible(None), quick),
            Next::Stop { line: None }
        );
        assert_eq!(
            tries.after("svc", &Ending::Impossible(Some("no bus".to_owned())), quick),
            Next::Stop {
                line: Some("sill: svc: no bus; it keeps its last value".to_owned())
            }
        );
    }
}
