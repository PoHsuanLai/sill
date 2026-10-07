//! The Now Playing position as a pure [`Machine`]: the last report from the player and when it
//! arrived, and the next whole second of the track as its wake. While the track plays the position
//! runs on from the report, one wake a second, on the second, until its end; held (paused or
//! buffering) or ended, the machine has no wake and nothing runs.

use crate::now_playing::kind::PositionClock;
use ds::base::machine::{Elapsed, Machine};
use ds::base::time::stamp::Stamp;
use std::time::Duration;

/// A position report: where, out of how long, and whether it runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Report {
    pub(crate) at: Duration,
    pub(crate) length: Duration,
    pub(crate) clock: PositionClock,
}

/// The last report, when it arrived, and when the position next reaches a whole second.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Ticker {
    report: Report,
    since: Stamp,
    due: Option<Stamp>,
}

/// What moves the ticker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TickIn {
    /// The player's report, passed on every render: a new one re-anchors the position, the same
    /// one again changes nothing.
    Report(Report),
    /// A whole second came.
    Elapsed,
}

impl From<Elapsed> for TickIn {
    fn from(_: Elapsed) -> Self {
        TickIn::Elapsed
    }
}

/// Where the track is `elapsed` after `report` arrived: running from it, or held at it, never
/// past the end.
pub(crate) fn position(report: Report, elapsed: Duration) -> Duration {
    let at = match report.clock {
        PositionClock::Running => report.at + elapsed,
        PositionClock::Held => report.at,
    };
    at.min(report.length)
}

/// How long until the position reaches its next whole second.
pub(crate) fn to_next_second(at: Duration) -> Duration {
    let into = u64::from(at.subsec_millis());
    Duration::from_millis(1000 - into)
}

impl Ticker {
    /// `report`, which arrived at `at`.
    pub(crate) fn arrived(report: Report, at: Stamp) -> Self {
        Ticker {
            report,
            since: at,
            due: next_second(report, at, at),
        }
    }

    /// Where the track is at `at`.
    pub(crate) fn position(self, at: Stamp) -> Duration {
        position(self.report, Duration::from_millis(at.since(self.since)))
    }

    /// Whether this is the report being followed.
    pub(crate) fn follows(self, report: Report) -> bool {
        self.report == report
    }
}

/// When the track next reaches a whole second, as of `at`; none once it is held or has ended.
fn next_second(report: Report, since: Stamp, at: Stamp) -> Option<Stamp> {
    let here = position(report, Duration::from_millis(at.since(since)));
    match report.clock {
        PositionClock::Held => None,
        PositionClock::Running if here >= report.length => None,
        PositionClock::Running => Some(at.after_span(to_next_second(here))),
    }
}

impl Machine for Ticker {
    type In = TickIn;
    type Out = ();
    type Params = ();
    type Ctx = ();

    fn step(self, input: TickIn, at: Stamp, _: &(), _: &()) -> (Ticker, Vec<()>) {
        let next = match (input, self.due) {
            (TickIn::Report(report), _) if report == self.report => self,
            (TickIn::Report(report), _) => Ticker::arrived(report, at),
            (TickIn::Elapsed, Some(due)) if at >= due => Ticker {
                due: next_second(self.report, self.since, at),
                ..self
            },
            (TickIn::Elapsed, Some(_) | None) => self,
        };
        (next, Vec::new())
    }

    fn wake(&self) -> Option<Stamp> {
        self.due
    }
}

#[cfg(test)]
mod tests {
    use super::{Report, TickIn, Ticker, position, to_next_second};
    use crate::now_playing::kind::PositionClock;
    use ds::base::machine::Machine;
    use ds::base::time::stamp::Stamp;
    use std::time::Duration;

    fn ms(n: u64) -> Duration {
        Duration::from_millis(n)
    }

    fn report(at: u64, length: u64, clock: PositionClock) -> Report {
        Report {
            at: ms(at),
            length: ms(length),
            clock,
        }
    }

    #[test]
    fn a_running_position_advances_from_its_report_and_stops_at_the_end() {
        let running = report(61_400, 200_000, PositionClock::Running);
        let held = Report {
            clock: PositionClock::Held,
            ..running
        };
        let cases = [
            (running, ms(0), ms(61_400)),
            (running, ms(600), ms(62_000)),
            (running, ms(500_000), ms(200_000)),
            (held, ms(5_000), ms(61_400)),
        ];
        for (report, elapsed, want) in cases {
            assert_eq!(position(report, elapsed), want, "{report:?} +{elapsed:?}");
        }
    }

    #[test]
    fn it_wakes_on_the_second() {
        assert_eq!(to_next_second(ms(61_400)), ms(600));
        assert_eq!(to_next_second(ms(62_000)), ms(1_000));
    }

    #[test]
    fn the_ticker_beats_on_each_second_of_the_track_until_it_ends_or_is_held() {
        let running = report(61_400, 63_000, PositionClock::Running);
        let held = Report {
            clock: PositionClock::Held,
            ..running
        };
        let arrived = |report, at| Ticker::arrived(report, Stamp(at));
        /// Name, state before, input, time (ms), the position after, next wake.
        type Case = (&'static str, Ticker, TickIn, u64, Duration, Option<u64>);
        let cases: &[Case] = &[
            (
                "a report arrives at 100: the next second is 600 ms on",
                arrived(running, 100),
                TickIn::Elapsed,
                100,
                ms(61_400),
                Some(700),
            ),
            (
                "woken early: nothing",
                arrived(running, 100),
                TickIn::Elapsed,
                699,
                ms(61_400 + 599),
                Some(700),
            ),
            (
                "on the second: the next is a second on",
                arrived(running, 100),
                TickIn::Elapsed,
                700,
                ms(62_000),
                Some(1_700),
            ),
            (
                "a late wake: the second after it is on the grid",
                arrived(running, 100),
                TickIn::Elapsed,
                710,
                ms(62_010),
                Some(1_700),
            ),
            (
                "the last second: the track ends, no more wakes",
                arrived(running, 100),
                TickIn::Elapsed,
                1_700,
                ms(63_000),
                None,
            ),
            (
                "held: no wake",
                arrived(held, 100),
                TickIn::Elapsed,
                700,
                ms(61_400),
                None,
            ),
            (
                "a new report re-anchors",
                arrived(running, 100),
                TickIn::Report(report(10_250, 63_000, PositionClock::Running)),
                5_000,
                ms(10_250),
                Some(5_750),
            ),
            (
                "the same report again: unchanged",
                arrived(running, 100),
                TickIn::Report(running),
                5_000,
                ms(63_000),
                Some(700),
            ),
            (
                "a report that holds it stops the wakes",
                arrived(running, 100),
                TickIn::Report(held),
                5_000,
                ms(61_400),
                None,
            ),
        ];
        for (name, from, input, at, shown, wake) in cases {
            let (next, out) = from.step(*input, Stamp(*at), &(), &());
            assert_eq!(next.position(Stamp(*at)), *shown, "{name}: position");
            assert!(out.is_empty(), "{name}: outputs");
            assert_eq!(next.wake(), wake.map(Stamp), "{name}: wake");
        }
    }
}
