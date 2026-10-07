//! Following a widget's timeline as a pure [`Machine`]: it wakes for the next entry's date (the
//! caller redraws with it) and for the refresh policy (it asks once and stops: the answer is a new
//! timeline, which starts a new follow). The next wake is in the state, so a timeline that is only
//! one entry, or has none left, runs no timer.
//!
//! A timeline's dates are instants on the design system's clock; the machine counts in
//! [`Stamp`]s, so its context is the [`FrameClock`] that turns one into the other.

use crate::widget::timeline::{RefreshAsk, Timeline, Wake};
use ds::base::machine::{Elapsed, Machine};
use ds::base::time::stamp::{FrameClock, Stamp};
use std::time::Instant;

/// What the follower wakes for next, and when.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Due {
    /// Nothing more happens.
    Never,
    /// The next entry's date.
    Entry(Stamp),
    /// The refresh, which asks this.
    Refresh(Stamp, RefreshAsk),
}

/// The timeline being followed, when it arrived, and what is next.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Follower<E> {
    timeline: Option<Timeline<E>>,
    arrived: Stamp,
    due: Due,
}

/// What moves the follower.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum FollowIn<E> {
    /// The provider's timeline, passed on every render; an equal one again changes nothing.
    Arrive(Timeline<E>),
    /// The next wake came.
    Elapsed,
}

impl<E> From<Elapsed> for FollowIn<E> {
    fn from(_: Elapsed) -> Self {
        FollowIn::Elapsed
    }
}

impl<E> Follower<E> {
    /// Following nothing yet.
    pub(crate) fn idle() -> Self {
        Follower {
            timeline: None,
            arrived: Stamp(0),
            due: Due::Never,
        }
    }
}

impl<E: PartialEq> Follower<E> {
    /// Whether `timeline` is the one being followed.
    pub(crate) fn follows(&self, timeline: &Timeline<E>) -> bool {
        self.timeline.as_ref() == Some(timeline)
    }
}

impl<E: Clone + PartialEq + 'static> Machine for Follower<E> {
    type In = FollowIn<E>;
    /// The refresh policy came due: ask the provider for a timeline.
    type Out = RefreshAsk;
    type Params = ();
    /// How the timeline's instants count in stamps.
    type Ctx = FrameClock;

    fn step(
        self,
        input: FollowIn<E>,
        at: Stamp,
        _: &(),
        clock: &FrameClock,
    ) -> (Follower<E>, Vec<RefreshAsk>) {
        match (input, self.timeline.as_ref(), self.due) {
            (FollowIn::Arrive(timeline), held, _) if held != Some(&timeline) => (
                Follower {
                    due: due_after(&timeline, at, at, clock),
                    timeline: Some(timeline),
                    arrived: at,
                },
                Vec::new(),
            ),
            (FollowIn::Elapsed, Some(timeline), Due::Entry(due)) if at >= due => (
                Follower {
                    due: due_after(timeline, at, self.arrived, clock),
                    ..self
                },
                Vec::new(),
            ),
            (FollowIn::Elapsed, _, Due::Refresh(due, ask)) if at >= due => (
                Follower {
                    due: Due::Never,
                    ..self
                },
                vec![ask],
            ),
            (FollowIn::Arrive(_) | FollowIn::Elapsed, _, _) => (self, Vec::new()),
        }
    }

    fn wake(&self) -> Option<Stamp> {
        match self.due {
            Due::Never => None,
            Due::Entry(at) | Due::Refresh(at, _) => Some(at),
        }
    }
}

/// What `timeline` wakes for after `now`, having arrived at `arrived`.
fn due_after<E>(timeline: &Timeline<E>, now: Stamp, arrived: Stamp, clock: &FrameClock) -> Due {
    match timeline.next_wake(clock.instant(now), clock.instant(arrived)) {
        None => Due::Never,
        Some(Wake::Entry(at)) => Due::Entry(reached_by(clock, at)),
        Some(Wake::Refresh(at, ask)) => Due::Refresh(reached_by(clock, at), ask),
    }
}

/// The first whole millisecond at or after `at`, so a wake at it finds the date reached.
fn reached_by(clock: &FrameClock, at: Instant) -> Stamp {
    let floor = clock.stamp(at);
    if clock.instant(floor) < at {
        floor.after(1)
    } else {
        floor
    }
}

#[cfg(test)]
mod tests {
    use super::{Due, FollowIn, Follower};
    use crate::widget::timeline::{Dated, EntryDate, Refresh, RefreshAsk, Timeline};
    use ds::base::machine::Machine;
    use ds::base::time::stamp::{FrameClock, Stamp};
    use std::time::{Duration, Instant};

    /// Entries 'a' now, 'b' at 60 s, 'c' at 120 s; the refresh policy as given.
    fn timeline(origin: Instant, refresh: Refresh) -> Timeline<char> {
        let at = |seconds| EntryDate::At(origin + Duration::from_secs(seconds));
        Timeline::new(
            vec![
                Dated::new(EntryDate::Start, 'a'),
                Dated::new(at(60), 'b'),
                Dated::new(at(120), 'c'),
            ],
            refresh,
        )
    }

    fn follower(timeline: Timeline<char>, arrived: u64, due: Due) -> Follower<char> {
        Follower {
            timeline: Some(timeline),
            arrived: Stamp(arrived),
            due,
        }
    }

    #[test]
    fn it_wakes_for_each_entry_date_then_for_the_refresh_once_and_then_rests() {
        let origin = Instant::now();
        let clock = FrameClock::new(origin);
        let ms = |seconds: u64| Stamp(seconds * 1000);
        let never = timeline(origin, Refresh::Never);
        let at_end = timeline(origin, Refresh::AtEnd);
        let arrive = |tl| FollowIn::Arrive(tl);
        /// Name, state before, input, time (ms), state after, what it asks, next wake.
        type Case = (
            &'static str,
            Follower<char>,
            FollowIn<char>,
            u64,
            Follower<char>,
            Vec<RefreshAsk>,
            Option<Stamp>,
        );
        let cases: Vec<Case> = vec![
            (
                "a timeline arrives: it wakes for the second entry",
                Follower::idle(),
                arrive(never.clone()),
                0,
                follower(never.clone(), 0, Due::Entry(ms(60))),
                vec![],
                Some(ms(60)),
            ),
            (
                "the same timeline again: unchanged",
                follower(never.clone(), 0, Due::Entry(ms(60))),
                arrive(never.clone()),
                5_000,
                follower(never.clone(), 0, Due::Entry(ms(60))),
                vec![],
                Some(ms(60)),
            ),
            (
                "an early wake: nothing",
                follower(never.clone(), 0, Due::Entry(ms(60))),
                FollowIn::Elapsed,
                59_999,
                follower(never.clone(), 0, Due::Entry(ms(60))),
                vec![],
                Some(ms(60)),
            ),
            (
                "the second entry's date: on to the third",
                follower(never.clone(), 0, Due::Entry(ms(60))),
                FollowIn::Elapsed,
                60_000,
                follower(never.clone(), 0, Due::Entry(ms(120))),
                vec![],
                Some(ms(120)),
            ),
            (
                "the last entry's date with no refresh: at rest",
                follower(never.clone(), 0, Due::Entry(ms(120))),
                FollowIn::Elapsed,
                120_000,
                follower(never.clone(), 0, Due::Never),
                vec![],
                None,
            ),
            (
                "with AtEnd: the refresh follows the last entry",
                follower(at_end.clone(), 0, Due::Entry(ms(120))),
                FollowIn::Elapsed,
                120_000,
                follower(at_end.clone(), 0, Due::Refresh(ms(120), RefreshAsk::Ended)),
                vec![],
                Some(ms(120)),
            ),
            (
                "the refresh asks once and rests",
                follower(at_end.clone(), 0, Due::Refresh(ms(120), RefreshAsk::Ended)),
                FollowIn::Elapsed,
                120_000,
                follower(at_end.clone(), 0, Due::Never),
                vec![RefreshAsk::Ended],
                None,
            ),
            (
                "a new timeline starts a new follow from its arrival",
                follower(at_end.clone(), 0, Due::Never),
                arrive(never.clone()),
                130_000,
                follower(never.clone(), 130_000, Due::Never),
                vec![],
                None,
            ),
        ];
        for (name, from, input, at, state, asks, wake) in cases {
            let (next, out) = from.step(input, Stamp(at), &(), &clock);
            assert_eq!(next, state, "{name}: state");
            assert_eq!(out, asks, "{name}: asks");
            assert_eq!(next.wake(), wake, "{name}: wake");
        }
    }

    #[test]
    fn a_wake_lands_on_the_first_whole_millisecond_that_has_reached_the_date() {
        let origin = Instant::now();
        let clock = FrameClock::new(origin);
        let late = EntryDate::At(origin + Duration::from_micros(60_000_400));
        let tl = Timeline::new(
            vec![Dated::new(EntryDate::Start, 'a'), Dated::new(late, 'b')],
            Refresh::Never,
        );
        let (state, _) = Follower::idle().step(FollowIn::Arrive(tl), Stamp(0), &(), &clock);
        assert_eq!(
            state.wake(),
            Some(Stamp(60_001)),
            "60 000.4 ms rounds up, not down"
        );
        let (after, _) = state.step(FollowIn::Elapsed, Stamp(60_001), &(), &clock);
        assert_eq!(
            after.wake(),
            None,
            "the date was reached: no second wake for it"
        );
    }
}
