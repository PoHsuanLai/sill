//! The World Clock widget (M10 lane b): one row per zone in `widgets.world_clocks`, the city,
//! the time (24-hour, like the bar clock) and how far it is from here ("Today, +7h",
//! "Tomorrow, +13h"), redrawn on the clock's minute tick. A zone jiff does not know is left out.
//! An analog face is a quire gap (Q183); the rows are text meanwhile.

use dioxus::prelude::*;
use jiff::Timestamp;
use jiff::tz::TimeZone;

use super::WidgetHost;

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
    /// `09:05`.
    pub time: String,
    /// `Today, +7h`, `Yesterday, -5h30`, `Tomorrow, +13h`, `Today, same time`.
    pub offset: String,
}

/// `widgets.world_clocks`: `;`-separated, trimmed, empty entries skipped, duplicates kept once.
pub fn parse_zones(text: &str) -> Vec<ZoneName> {
    let _ = text;
    todo!("M10 fill: b-widgets")
}

/// The rows for `zones` at `now`, relative to `here`; unknown zones left out.
pub fn clock_rows(now: Timestamp, zones: &[ZoneName], here: &TimeZone) -> Vec<ClockRow> {
    let _ = (now, zones, here);
    todo!("M10 fill: b-widgets")
}

/// The clocks.
#[component]
pub fn WorldClockWidget(host: WidgetHost) -> Element {
    let _ = host;
    rsx! {}
}

#[cfg(test)]
mod tests {
    use jiff::civil::date;
    use jiff::tz::TimeZone;

    use super::{ClockRow, ZoneName, clock_rows, parse_zones};

    #[test]
    #[ignore = "M10 fill: b-widgets"]
    fn zones_parse() {
        let zone = |z: &str| ZoneName(z.to_owned());
        assert_eq!(
            parse_zones(" Europe/London ;;America/New_York; Europe/London"),
            vec![zone("Europe/London"), zone("America/New_York")]
        );
        assert_eq!(parse_zones(""), Vec::new());
    }

    #[test]
    #[ignore = "M10 fill: b-widgets"]
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
        let row = |city: &str, time: &str, offset: &str| ClockRow {
            city: city.to_owned(),
            time: time.to_owned(),
            offset: offset.to_owned(),
        };
        assert_eq!(
            clock_rows(now, &zones, &here),
            vec![
                row("London", "03:00", "Today, -7h"),
                row("Auckland", "14:00", "Today, +4h"),
                row("Taipei", "10:00", "Today, same time"),
            ]
        );
    }
}
