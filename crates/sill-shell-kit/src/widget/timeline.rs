//! A widget's timeline (design/23-WIDGETS.md section 9.2): the entries a provider hands over at
//! once, each from its date on, and when to ask for the next timeline. The logic here is pure: it
//! takes "now" as an argument, so a table test needs no clock;
//! [`crate::widget::use_widget::use_widget`] reads the
//! design system's clock and sleeps on it (the virtual clock in a test).

use std::time::{Duration, Instant};

/// From when an entry shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum EntryDate {
    /// From the moment the timeline arrives.
    Start,
    /// From this instant on the design system's clock (`ds::base::time::clock::now`).
    At(Instant),
}

impl EntryDate {
    /// Whether the entry shows at `now`.
    fn reached(self, now: Instant) -> Reached {
        match self {
            EntryDate::Start => Reached::Yes,
            EntryDate::At(at) if at <= now => Reached::Yes,
            EntryDate::At(_) => Reached::No,
        }
    }
}

/// Whether a date has come.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Reached {
    Yes,
    No,
}

/// An entry and its date.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Dated<E> {
    /// From when it shows.
    pub date: EntryDate,
    /// What it shows.
    pub entry: E,
}

impl<E> Dated<E> {
    /// `entry` from `date` on.
    pub fn new(date: EntryDate, entry: E) -> Self {
        Dated { date, entry }
    }
}

/// When the host asks the provider for the next timeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Refresh {
    /// Never on its own: the provider sends a new timeline when its data changes (a battery
    /// level, a new track). What a live, pushed widget uses.
    #[default]
    Never,
    /// When the last entry's date has come (a clock that sent the next hour of minutes).
    AtEnd,
    /// At this instant, whatever the entries.
    After(Instant),
}

/// Why the host asks for a new timeline: handed to the card's `onrefresh`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RefreshAsk {
    /// [`Refresh::AtEnd`]: the last entry is showing.
    Ended,
    /// [`Refresh::After`]: its instant has come.
    Due,
}

/// The soonest the host asks for a new timeline after one arrives, whatever its policy says: a
/// provider that answers every ask with a timeline already due cannot spin the host.
pub const REFRESH_FLOOR: Duration = Duration::from_secs(1);

/// A provider's entries, in date order, and its refresh policy.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Timeline<E> {
    entries: Vec<Dated<E>>,
    refresh: Refresh,
}

impl<E> Default for Timeline<E> {
    /// No entries: the widget shows its placeholder.
    fn default() -> Self {
        Timeline {
            entries: Vec::new(),
            refresh: Refresh::Never,
        }
    }
}

impl<E> Timeline<E> {
    /// `entries` (sorted here by date; the order given for one date is kept) and `refresh`.
    pub fn new(entries: Vec<Dated<E>>, refresh: Refresh) -> Self {
        let mut entries = entries;
        entries.sort_by_key(|dated| dated.date);
        Timeline { entries, refresh }
    }

    /// One entry, shown from arrival until the provider sends another: a live widget's
    /// timeline.
    pub fn now(entry: E) -> Self {
        Timeline::new(vec![Dated::new(EntryDate::Start, entry)], Refresh::Never)
    }

    /// The entries, in date order.
    pub fn entries(&self) -> &[Dated<E>] {
        &self.entries
    }

    /// The refresh policy.
    pub fn refresh(&self) -> Refresh {
        self.refresh
    }

    /// The entry showing at `now`: the last whose date has come, or `None` before the first
    /// (the widget then shows its placeholder).
    pub fn current(&self, now: Instant) -> Option<&E> {
        self.entries
            .iter()
            .take_while(|dated| dated.date.reached(now) == Reached::Yes)
            .last()
            .map(|dated| &dated.entry)
    }

    /// The next thing to wake for after `now`, the timeline having arrived at `arrived`: the
    /// next entry's date, or the refresh (never before [`REFRESH_FLOOR`] after arrival),
    /// whichever is first; `None` when nothing more happens.
    pub fn next_wake(&self, now: Instant, arrived: Instant) -> Option<Wake> {
        let entry = self.entries.iter().find_map(|dated| match dated.date {
            EntryDate::At(at) if at > now => Some(Wake::Entry(at)),
            EntryDate::At(_) | EntryDate::Start => None,
        });
        let floor = arrived + REFRESH_FLOOR;
        let refresh = match self.refresh {
            Refresh::Never => None,
            Refresh::AtEnd => Some(Wake::Refresh(
                self.last_date(arrived).max(floor),
                RefreshAsk::Ended,
            )),
            Refresh::After(at) => Some(Wake::Refresh(at.max(floor), RefreshAsk::Due)),
        };
        match (entry, refresh) {
            (Some(entry), Some(refresh)) if refresh.at() < entry.at() => Some(refresh),
            (Some(entry), _) => Some(entry),
            (None, refresh) => refresh,
        }
    }

    /// The last entry's date, a `Start` (or no entry) read as `arrived`.
    fn last_date(&self, arrived: Instant) -> Instant {
        match self.entries.last().map(|dated| dated.date) {
            Some(EntryDate::At(at)) => at,
            Some(EntryDate::Start) | None => arrived,
        }
    }
}

/// What the host wakes for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Wake {
    /// An entry's date: redraw.
    Entry(Instant),
    /// The refresh: ask the provider, then wait for its answer.
    Refresh(Instant, RefreshAsk),
}

impl Wake {
    /// When.
    pub fn at(self) -> Instant {
        match self {
            Wake::Entry(at) | Wake::Refresh(at, _) => at,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Dated, EntryDate, REFRESH_FLOOR, Refresh, RefreshAsk, Timeline, Wake};
    use std::time::{Duration, Instant};

    fn at(origin: Instant, seconds: u64) -> EntryDate {
        EntryDate::At(origin + Duration::from_secs(seconds))
    }

    #[test]
    fn the_current_entry_is_the_last_whose_date_has_come() {
        let o = Instant::now();
        let timeline = Timeline::new(
            vec![
                Dated::new(at(o, 60), 'b'),
                Dated::new(at(o, 0), 'a'),
                Dated::new(at(o, 120), 'c'),
            ],
            Refresh::Never,
        );
        let s = |n: u64| o + Duration::from_secs(n);
        assert_eq!(timeline.current(s(0)), Some(&'a'));
        assert_eq!(timeline.current(s(59)), Some(&'a'));
        assert_eq!(timeline.current(s(60)), Some(&'b'));
        assert_eq!(timeline.current(s(500)), Some(&'c'));
        let later = Timeline::new(vec![Dated::new(at(o, 10), 'x')], Refresh::Never);
        assert_eq!(
            later.current(s(9)),
            None,
            "before the first entry: the placeholder"
        );
        assert_eq!(Timeline::now('n').current(s(0)), Some(&'n'));
        assert_eq!(Timeline::<char>::default().current(s(0)), None);
    }

    #[test]
    fn the_host_wakes_for_each_entry_then_the_refresh() {
        let o = Instant::now();
        let s = |n: u64| o + Duration::from_secs(n);
        let timeline = Timeline::new(
            vec![
                Dated::new(at(o, 0), 1),
                Dated::new(at(o, 60), 2),
                Dated::new(at(o, 120), 3),
            ],
            Refresh::AtEnd,
        );
        let cases = [
            (0, Some(Wake::Entry(s(60)))),
            (60, Some(Wake::Entry(s(120)))),
            (120, Some(Wake::Refresh(s(120), RefreshAsk::Ended))),
        ];
        for (now, want) in cases {
            assert_eq!(timeline.next_wake(s(now), o), want, "at {now}");
        }
        let never = Timeline::new(vec![Dated::new(at(o, 0), 1)], Refresh::Never);
        assert_eq!(never.next_wake(s(5), o), None);
        let due = Timeline::new(
            vec![Dated::new(at(o, 0), 1), Dated::new(at(o, 90), 2)],
            Refresh::After(s(30)),
        );
        assert_eq!(
            due.next_wake(s(0), o),
            Some(Wake::Refresh(s(30), RefreshAsk::Due))
        );
    }

    #[test]
    fn a_refresh_already_due_waits_for_the_floor() {
        let o = Instant::now();
        let live = Timeline::new(vec![Dated::new(EntryDate::Start, 1)], Refresh::AtEnd);
        assert_eq!(
            live.next_wake(o, o),
            Some(Wake::Refresh(o + REFRESH_FLOOR, RefreshAsk::Ended))
        );
        let past = Timeline::new(vec![Dated::new(EntryDate::Start, 1)], Refresh::After(o));
        assert_eq!(
            past.next_wake(o, o),
            Some(Wake::Refresh(o + REFRESH_FLOOR, RefreshAsk::Due))
        );
    }
}
