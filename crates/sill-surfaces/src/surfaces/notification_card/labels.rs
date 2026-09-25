//! The words on a card: the app's name, how long ago it arrived, and the count chip.

use sill_services::{AppRef, UnixMs};

/// How long ago a notification arrived, as a card shows it (design/13 §13.3.6: "time data 10
/// top-right").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Age {
    /// Under a minute.
    Now,
    /// Whole minutes, under an hour.
    Minutes(u32),
    /// Whole hours, under a day.
    Hours(u32),
    /// Whole days.
    Days(u32),
}

/// `arrived`'s age at `now` (a clock that went backwards reads as now).
pub fn age(arrived: UnixMs, now: UnixMs) -> Age {
    const MINUTE: u64 = 60_000;
    const HOUR: u64 = 60 * MINUTE;
    const DAY: u64 = 24 * HOUR;
    let whole = |unit: u64| u32::try_from(now.since(arrived) / unit).unwrap_or(u32::MAX);
    match now.since(arrived) {
        ms if ms < MINUTE => Age::Now,
        ms if ms < HOUR => Age::Minutes(whole(MINUTE)),
        ms if ms < DAY => Age::Hours(whole(HOUR)),
        _ => Age::Days(whole(DAY)),
    }
}

/// `age` as the card writes it: `now`, `5m`, `2h`, `3d` (macOS's compact form).
pub fn age_label(age: Age) -> String {
    match age {
        Age::Now => "now".to_owned(),
        Age::Minutes(n) => format!("{n}m"),
        Age::Hours(n) => format!("{n}h"),
        Age::Days(n) => format!("{n}d"),
    }
}

/// The count chip's text: none for a lone notification, else the count (`99+` past 99).
pub fn count_label(count: usize) -> Option<String> {
    match count {
        0 | 1 => None,
        2..=99 => Some(count.to_string()),
        _ => Some("99+".to_owned()),
    }
}

/// The app's name as drawn: its desktop entry's `Name` when the apps index knows it, else the
/// sender's `app_name`, else "Notification".
pub fn app_label(app: &AppRef, entry_name: Option<&str>) -> String {
    [entry_name.unwrap_or_default(), app.name.as_str()]
        .into_iter()
        .map(str::trim)
        .find(|name| !name.is_empty())
        .unwrap_or("Notification")
        .to_owned()
}

#[cfg(test)]
mod tests {
    use super::{Age, age, age_label, app_label, count_label};
    use sill_services::{AppRef, IconRef, UnixMs};

    #[test]
    fn ages_round_down_to_the_unit() {
        const MIN: i64 = 60_000;
        let cases = [
            (0, Age::Now),
            (59_999, Age::Now),
            (MIN, Age::Minutes(1)),
            (59 * MIN + 59_999, Age::Minutes(59)),
            (60 * MIN, Age::Hours(1)),
            (24 * 60 * MIN - 1, Age::Hours(23)),
            (24 * 60 * MIN, Age::Days(1)),
            (-5_000, Age::Now),
        ];
        for (elapsed, want) in cases {
            assert_eq!(
                age(UnixMs(1_000_000), UnixMs(1_000_000 + elapsed)),
                want,
                "{elapsed}"
            );
        }
        let labels = [
            (Age::Now, "now"),
            (Age::Minutes(5), "5m"),
            (Age::Hours(2), "2h"),
            (Age::Days(3), "3d"),
        ];
        for (a, want) in labels {
            assert_eq!(age_label(a), want, "{a:?}");
        }
    }

    #[test]
    fn the_chip_and_the_name() {
        assert_eq!(count_label(1), None);
        assert_eq!(count_label(3).as_deref(), Some("3"));
        assert_eq!(count_label(150).as_deref(), Some("99+"));
        let app = |name: &str| AppRef {
            name: name.to_owned(),
            icon: IconRef::None,
            desktop_entry: None,
        };
        assert_eq!(app_label(&app("evolution"), Some("Evolution")), "Evolution");
        assert_eq!(app_label(&app("evolution"), None), "evolution");
        assert_eq!(app_label(&app(""), None), "Notification");
    }
}
