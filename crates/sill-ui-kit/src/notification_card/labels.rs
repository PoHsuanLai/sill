//! The words on a card: the app's name, how long ago it arrived, and the group's count (quire
//! draws the chip from two and the layers behind the card).

use sill_model::notifications::{AppRef, UnixMs};
use sill_shell_kit::notifications::parts::{GroupCount, StackLayers};

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

/// The most layers a group draws behind its card (design/13 §13.3.6 draws two).
const GROUP_LAYERS: usize = 2;

/// A card standing for `count` notifications: nothing alone; else the count and one layer per
/// notification behind the newest, at most [`GROUP_LAYERS`].
pub fn group_count(count: usize) -> Option<GroupCount> {
    let layers = u8::try_from(count.saturating_sub(1).min(GROUP_LAYERS)).unwrap_or(0);
    (count > 1).then(|| GroupCount {
        count: u32::try_from(count).unwrap_or(u32::MAX),
        layers: StackLayers(layers),
    })
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
    use super::{Age, age, age_label, app_label, group_count};
    use sill_model::notifications::{AppRef, IconRef, UnixMs};
    use sill_shell_kit::notifications::parts::{GroupCount, StackLayers};

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
    fn a_group_counts_and_draws_at_most_two_layers() {
        let group = |count, layers| {
            Some(GroupCount {
                count,
                layers: StackLayers(layers),
            })
        };
        let cases = [
            (0, None),
            (1, None),
            (2, group(2, 1)),
            (3, group(3, 2)),
            (150, group(150, 2)),
        ];
        for (count, want) in cases {
            assert_eq!(group_count(count), want, "{count}");
        }
    }

    #[test]
    fn the_name_is_the_entrys_then_the_senders() {
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
