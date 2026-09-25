//! The World Clock widget (M10 lane b): a quire `ClockFace` (Q183) per zone in
//! `widgets.world_clocks`, the city under it and how far it is from here ("Today, +7h",
//! "Tomorrow, +13h"), redrawn on the clock's minute tick. A zone jiff does not know is left out.

use dioxus::prelude::*;
use ds::{ClockFace, ClockLook, ClockTime, DayPhase, Seconds};
use jiff::Timestamp;
use jiff::tz::TimeZone;

use super::WidgetHost;
use crate::hooks::{use_clock, use_settings};

/// This widget's stylesheet, in [`crate::widgets::widgets_css`].
pub const CSS: &str = include_str!("../style/widget_world_clock.css");

/// An IANA zone name: `Europe/London`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ZoneName(pub String);

/// One row.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ClockRow {
    /// The zone's last segment, underscores as spaces: `New York`.
    pub city: String,
    /// The wall-clock time there, to the minute.
    pub time: ClockTime,
    /// Day or night there, which tints the dial.
    pub phase: DayPhase,
    /// `Today, +7h`, `Yesterday, -5h30`, `Tomorrow, +13h`, `Today, same time`.
    pub offset: String,
}

/// The first hour a dial reads as day, and the first it reads as night again (macOS turns its
/// world clock's face dark from 18:00 to 06:00).
const DAY_HOURS: std::ops::Range<i8> = 6..18;

/// Day or night at `hour` (0 to 23) there.
pub fn phase_of(hour: i8) -> DayPhase {
    match DAY_HOURS.contains(&hour) {
        true => DayPhase::Day,
        false => DayPhase::Night,
    }
}

/// The wall-clock time of `hour` and `minute` as jiff hands them, the seconds hidden.
fn clock_time(hour: i8, minute: i8) -> ClockTime {
    let unsigned = |value: i8| u8::try_from(value).unwrap_or_default();
    ClockTime {
        hour: unsigned(hour),
        minute: unsigned(minute),
        second: Seconds::Hidden,
    }
}

/// `widgets.world_clocks`: `;`-separated, trimmed, empty entries skipped, duplicates kept once.
pub fn parse_zones(text: &str) -> Vec<ZoneName> {
    text.split(';')
        .map(str::trim)
        .filter(|zone| !zone.is_empty())
        .fold(Vec::new(), |mut zones, zone| {
            let zone = ZoneName(zone.to_owned());
            if !zones.contains(&zone) {
                zones.push(zone);
            }
            zones
        })
}

/// The rows for `zones` at `now`, relative to `here`; unknown zones left out.
pub fn clock_rows(now: Timestamp, zones: &[ZoneName], here: &TimeZone) -> Vec<ClockRow> {
    let local = now.to_zoned(here.clone());
    zones
        .iter()
        .filter_map(|zone| {
            let there = now.to_zoned(TimeZone::get(&zone.0).ok()?);
            let apart = there.offset().seconds() - local.offset().seconds();
            let day = day_word(there.date().cmp(&local.date()));
            Some(ClockRow {
                city: city_of(zone),
                time: clock_time(there.hour(), there.minute()),
                phase: phase_of(there.hour()),
                offset: format!("{day}, {}", offset_text(apart)),
            })
        })
        .collect()
}

/// The zone's last segment, underscores as spaces.
fn city_of(zone: &ZoneName) -> String {
    zone.0
        .rsplit('/')
        .next()
        .unwrap_or(&zone.0)
        .replace('_', " ")
}

/// The zone's calendar day against here's.
fn day_word(there: std::cmp::Ordering) -> &'static str {
    match there {
        std::cmp::Ordering::Less => "Yesterday",
        std::cmp::Ordering::Equal => "Today",
        std::cmp::Ordering::Greater => "Tomorrow",
    }
}

/// `+7h`, `-5h30`, `same time`.
fn offset_text(seconds: i32) -> String {
    if seconds == 0 {
        return "same time".to_owned();
    }
    let sign = if seconds < 0 { '-' } else { '+' };
    let minutes = seconds.unsigned_abs() / 60;
    match (minutes / 60, minutes % 60) {
        (hours, 0) => format!("{sign}{hours}h"),
        (hours, rest) => format!("{sign}{hours}h{rest:02}"),
    }
}

/// How many cities a medium widget has room for, side by side (macOS's medium World Clock).
pub const MAX_CITIES: usize = 4;

/// How `host` draws a clock: dials on the desktop, digits in the notification center's tile.
pub fn look_of(host: WidgetHost) -> ClockLook {
    match host {
        WidgetHost::Desktop => ClockLook::Analog,
        WidgetHost::Tile => ClockLook::Digital,
    }
}

/// The clocks: one quire `ClockFace` per zone, up to [`MAX_CITIES`] side by side (a dial tinted
/// for day or night there on the desktop, digits that bump on each new minute in the tile), the
/// city under it and the day and offset under that. Redrawn on the clock service's minute tick
/// and never otherwise. No zones: a quiet line naming the key.
#[component]
pub fn WorldClockWidget(host: WidgetHost) -> Element {
    // The minute tick is the only thing that redraws the widget: no timer of its own.
    let _minute = use_clock()();
    let zones = parse_zones(&use_settings()().widgets.world_clocks);
    let rows = clock_rows(Timestamp::now(), &zones, &TimeZone::system());
    let look = look_of(host);
    rsx! {
        div { class: "sill-widget-world-clock", "data-host": host.slug(),
            if rows.is_empty() {
                span { class: "sill-widget-quiet", "No cities: add them in widgets.world_clocks" }
            }
            for row in rows.into_iter().take(MAX_CITIES) {
                div { key: "{row.city}", class: "sill-widget-city",
                    ClockFace { time: row.time, phase: row.phase, look, label: row.city.clone() }
                    span { class: "sill-widget-city-offset", {row.offset} }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use jiff::civil::date;
    use jiff::tz::TimeZone;

    use ds::{ClockLook, DayPhase};

    use super::{
        ClockRow, WidgetHost, ZoneName, clock_rows, look_of, offset_text, parse_zones, phase_of,
    };

    #[test]
    fn zones_parse() {
        let zone = |z: &str| ZoneName(z.to_owned());
        assert_eq!(
            parse_zones(" Europe/London ;;America/New_York; Europe/London"),
            vec![zone("Europe/London"), zone("America/New_York")]
        );
        assert_eq!(parse_zones(""), Vec::new());
    }

    #[test]
    fn offsets_read_in_hours_and_minutes() {
        let cases = [
            (0, "same time"),
            (7 * 3600, "+7h"),
            (-7 * 3600, "-7h"),
            (-(5 * 3600 + 1800), "-5h30"),
            (5 * 3600 + 2700, "+5h45"),
            (1800, "+0h30"),
        ];
        for (seconds, want) in cases {
            assert_eq!(offset_text(seconds), want, "{seconds}");
        }
    }

    #[test]
    fn rows_say_the_time_and_the_day_there() {
        let here = TimeZone::get("Asia/Taipei").unwrap_or_else(|e| panic!("{e}"));
        let now = date(2026, 9, 26)
            .at(10, 0, 0, 0)
            .to_zoned(here.clone())
            .map(|z| z.timestamp())
            .unwrap_or_else(|e| panic!("{e}"));
        let zones = [
            ZoneName("Europe/London".to_owned()),
            ZoneName("Pacific/Auckland".to_owned()),
            ZoneName("Nowhere/Atlantis".to_owned()),
            ZoneName("Asia/Taipei".to_owned()),
        ];
        let got: Vec<_> = clock_rows(now, &zones, &here)
            .into_iter()
            .map(|r: ClockRow| (r.city, r.time.digits(), r.phase, r.offset))
            .collect();
        let row = |city: &str, time: &str, phase, offset: &str| {
            (city.to_owned(), time.to_owned(), phase, offset.to_owned())
        };
        assert_eq!(
            got,
            vec![
                row("London", "03:00", DayPhase::Night, "Today, -7h"),
                row("Auckland", "14:00", DayPhase::Day, "Today, +4h"),
                row("Taipei", "10:00", DayPhase::Day, "Today, same time"),
            ]
        );
    }

    #[test]
    fn a_dial_is_day_from_six_to_six() {
        let cases = [
            (0, DayPhase::Night),
            (5, DayPhase::Night),
            (6, DayPhase::Day),
            (12, DayPhase::Day),
            (17, DayPhase::Day),
            (18, DayPhase::Night),
            (23, DayPhase::Night),
        ];
        for (hour, want) in cases {
            assert_eq!(phase_of(hour), want, "{hour}");
        }
    }

    #[test]
    fn the_desktop_draws_dials_and_the_tile_digits() {
        let cases = [
            (WidgetHost::Desktop, ClockLook::Analog),
            (WidgetHost::Tile, ClockLook::Digital),
        ];
        for (host, want) in cases {
            assert_eq!(look_of(host), want, "{host:?}");
        }
    }

    #[test]
    fn the_day_there_is_named_across_midnight() {
        let here = TimeZone::get("Europe/London").unwrap_or_else(|e| panic!("{e}"));
        let now = date(2026, 9, 26)
            .at(23, 30, 0, 0)
            .to_zoned(here.clone())
            .map(|z| z.timestamp())
            .unwrap_or_else(|e| panic!("{e}"));
        let zones = [
            ZoneName("Asia/Kolkata".to_owned()),
            ZoneName("America/Los_Angeles".to_owned()),
            ZoneName("Pacific/Kiritimati".to_owned()),
        ];
        let got: Vec<_> = clock_rows(now, &zones, &here)
            .into_iter()
            .map(|r| (r.city, r.time.digits(), r.offset))
            .collect();
        let want = |city: &str, time: &str, offset: &str| {
            (city.to_owned(), time.to_owned(), offset.to_owned())
        };
        assert_eq!(
            got,
            vec![
                want("Kolkata", "04:00", "Tomorrow, +4h30"),
                want("Los Angeles", "15:30", "Today, -8h"),
                want("Kiritimati", "12:30", "Tomorrow, +13h"),
            ]
        );
        let tokyo = TimeZone::get("Asia/Tokyo").unwrap_or_else(|e| panic!("{e}"));
        let rows = clock_rows(now, &[ZoneName("Europe/London".to_owned())], &tokyo);
        assert_eq!(
            rows.first().map(|r| r.offset.as_str()),
            Some("Yesterday, -8h")
        );
    }
}
