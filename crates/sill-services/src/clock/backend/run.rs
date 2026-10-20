//! The clock's task: sleep to each minute boundary and publish the formatted time.

use std::sync::Arc;

use sill_model::clock::{ClockCommand, ClockMinute, until_next_minute};
use tokio::sync::{mpsc, watch};

use crate::env::WallClock;

/// Why the task woke.
enum Wake {
    Minute,
    Refresh,
    /// Every control is gone: keep ticking on the timer alone.
    Deaf,
}

/// Publish the minute at each minute boundary and on `Refresh`, until every receiver is gone.
pub(crate) async fn run(
    tx: watch::Sender<ClockMinute>,
    mut commands: mpsc::UnboundedReceiver<ClockCommand>,
    clock: Arc<dyn WallClock>,
) {
    let mut listening = true;
    while !tx.is_closed() {
        let wait = until_next_minute(clock.now().time());
        let wake = if listening {
            tokio::select! {
                () = tokio::time::sleep(wait) => Wake::Minute,
                command = commands.recv() => match command {
                    Some(ClockCommand::Refresh) => Wake::Refresh,
                    None => Wake::Deaf,
                },
            }
        } else {
            tokio::time::sleep(wait).await;
            Wake::Minute
        };
        if matches!(wake, Wake::Deaf) {
            listening = false;
        }
        let minute = ClockMinute(clock.now().datetime());
        tx.send_if_modified(|shown| publish(shown, minute));
    }
}

/// Replace `shown` with `minute`; whether it changed (a `Refresh` inside the same minute does
/// not wake the bar).
fn publish(shown: &mut ClockMinute, minute: ClockMinute) -> bool {
    if *shown == minute {
        return false;
    }
    *shown = minute;
    true
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::publish;
    use crate::clock::ClockService;
    use crate::env::FixedClock;
    use crate::service::spawn;
    use sill_model::clock::{ClockCommand, ClockMinute};
    use sill_testkit::TempDir;

    #[test]
    fn only_a_new_minute_wakes_the_bar() {
        let minute = |m: i8| ClockMinute(jiff::civil::date(2026, 9, 24).at(14, m, 0, 0));
        let cases = [("same minute", 5, 5, false), ("next minute", 5, 6, true)];
        for (name, before, after, changed) in cases {
            let mut shown = minute(before);
            assert_eq!(publish(&mut shown, minute(after)), changed, "{name}");
            assert_eq!(shown, minute(after), "{name}");
        }
    }

    #[test]
    fn the_first_value_is_the_clocks_time_and_refresh_is_accepted() {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .build()
            .expect("a current-thread runtime builds"); // test-only
        let at = jiff::civil::date(2026, 9, 24)
            .at(14, 5, 0, 0)
            .to_zoned(jiff::tz::TimeZone::UTC)
            .expect("a valid instant"); // test-only
        let scratch = TempDir::new("clock");
        let env = crate::env::Env {
            clock: Arc::new(FixedClock(at)),
            ..crate::env::Env::isolated(scratch.path())
        };
        let clock = spawn::<ClockService>(rt.handle(), &env, ());
        assert_eq!(
            *clock.state.borrow(),
            ClockMinute(jiff::civil::date(2026, 9, 24).at(14, 5, 0, 0))
        );
        assert!(clock.control.send(ClockCommand::Refresh).is_ok());
    }
}
