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
    let _ = (arrived, now);
    todo!("M6 fill: b-banners")
}

/// `age` as the card writes it: `now`, `5m`, `2h`, `3d` (macOS's compact form).
pub fn age_label(age: Age) -> String {
    let _ = age;
    todo!("M6 fill: b-banners")
}

/// The count chip's text: none for a lone notification, else the count (`99+` past 99).
pub fn count_label(count: usize) -> Option<String> {
    let _ = count;
    todo!("M6 fill: b-banners")
}

/// The app's name as drawn: its desktop entry's `Name` when the apps index knows it, else the
/// sender's `app_name`, else "Notification".
pub fn app_label(app: &AppRef, entry_name: Option<&str>) -> String {
    let _ = (app, entry_name);
    todo!("M6 fill: b-banners")
}

#[cfg(test)]
mod tests {
    use super::{Age, age, age_label, app_label, count_label};
    use sill_services::{AppRef, IconRef, UnixMs};

    #[test]
    #[ignore = "M6 fill: b-banners"]
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
    #[ignore = "M6 fill: b-banners"]
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
