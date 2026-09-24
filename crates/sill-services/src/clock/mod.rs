//! The bar clock: a task that sleeps to each minute boundary and publishes the formatted time
//! (plan hooks table, `use_clock`: "sleep to minute boundary; pure format_clock").

mod format;

pub use format::{ClockText, format_clock, until_next_minute};

use jiff::Zoned;
use jiff::tz::TimeZone;
use tokio::runtime::Handle;
use tokio::sync::{mpsc, watch};

use crate::control::Control;

/// What the clock service can be told.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClockCommand {
    /// Re-read the time zone and reformat now (after a resume or a time-zone change).
    Refresh,
}

/// Start the clock task on `rt`: formats the local time now, then at every minute boundary.
pub fn spawn(rt: &Handle) -> (watch::Receiver<ClockText>, Control<ClockCommand>) {
    let (tx, rx) = watch::channel(format_clock(local_now().datetime()));
    let (control, commands) = Control::channel();
    rt.spawn(run(tx, commands));
    (rx, control)
}

/// The local wall clock, reading the system time zone afresh each time so a time-zone change
/// shows at the next minute (or at once on `Refresh`).
fn local_now() -> Zoned {
    let zone = TimeZone::try_system().unwrap_or(TimeZone::UTC);
    Zoned::now().with_time_zone(zone)
}

/// Why the task woke.
enum Wake {
    Minute,
    Refresh,
    /// Every control is gone: keep ticking on the timer alone.
    Deaf,
}

/// Publish the time at each minute boundary and on `Refresh`, until every receiver is gone.
async fn run(tx: watch::Sender<ClockText>, mut commands: mpsc::UnboundedReceiver<ClockCommand>) {
    let mut listening = true;
    while !tx.is_closed() {
        let wait = until_next_minute(local_now().time());
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
        let text = format_clock(local_now().datetime());
        tx.send_if_modified(|shown| publish(shown, text));
    }
}

/// Replace `shown` with `text`; whether it changed (a `Refresh` inside the same minute does
/// not wake the bar).
fn publish(shown: &mut ClockText, text: ClockText) -> bool {
    if *shown == text {
        return false;
    }
    *shown = text;
    true
}

#[cfg(test)]
mod tests {
    use super::{ClockCommand, ClockText, publish, spawn};

    #[test]
    fn only_a_new_minute_wakes_the_bar() {
        let text = |t: &str| ClockText { text: t.to_owned() };
        let cases = [
            ("same minute", "Thu 24 Sep 14:05", "Thu 24 Sep 14:05", false),
            ("next minute", "Thu 24 Sep 14:05", "Thu 24 Sep 14:06", true),
        ];
        for (name, before, after, changed) in cases {
            let mut shown = text(before);
            assert_eq!(publish(&mut shown, text(after)), changed, "{name}");
            assert_eq!(shown, text(after), "{name}");
        }
    }

    #[test]
    fn the_first_value_is_the_time_now_and_refresh_is_accepted() {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .build()
            .expect("a current-thread runtime builds"); // test-only
        let (rx, control) = spawn(rt.handle());
        let shown = rx.borrow().text.clone();
        // `Thu 24 Sep 14:05`: a weekday, a day, a month and HH:MM.
        let words: Vec<&str> = shown.split_whitespace().collect();
        assert_eq!(words.len(), 4, "{shown}");
        assert_eq!(words[3].len(), 5, "{shown}");
        assert!(control.send(ClockCommand::Refresh).is_ok());
    }
}
