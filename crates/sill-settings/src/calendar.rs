//! `calendar.*`: where the calendar widgets read events, and how the month is drawn (design/20
//! §1.12, which sill moves into the notification center's widgets as macOS 11 does). All Advanced.

use crate::file::ShellFile;
use crate::section::Section;
use ds_core::word::Word;
use ds_settings::Count;
use ds_settings::schema::Page;
use serde::{Deserialize, Serialize};

/// The first column of the month grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, Word)]
#[serde(rename_all = "snake_case")]
#[word(case = snake)]
pub enum FirstWeekday {
    /// ISO 8601, and the week numbers' own week.
    #[default]
    Monday,
    Sunday,
}

/// Whether the month grid has a column of ISO week numbers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, Word)]
#[serde(rename_all = "snake_case")]
#[word(case = snake)]
pub enum WeekNumbers {
    #[default]
    Hide,
    Show,
}

/// The calendar widgets.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ds_settings::SettingsSchema)]
#[serde(default)]
#[settings(file = "sill/settings.toml", domain = "calendar", page = Page::Notifications)]
pub struct CalendarSettings {
    /// `auto`, `off`, or `;`-separated `.ics` files and directories (parsed by
    /// `sill_model::events::parse_sources`).
    #[settings(
        label = "Calendars",
        help = "`auto` reads `~/.calendars` (vdirsyncer and khal) and `~/.local/share/sill/calendars`; `off` reads none; or `.ics` files and folders separated by `;`.",
        section = "Calendar",
        advanced
    )]
    pub sources: String,
    #[settings(
        label = "First day of the week",
        help = "The first column of the month.",
        section = "Calendar",
        advanced
    )]
    pub first_weekday: FirstWeekday,
    #[settings(
        label = "Week numbers",
        help = "A column of ISO week numbers beside the month.",
        section = "Calendar",
        advanced
    )]
    pub week_numbers: WeekNumbers,
    #[settings(
        label = "Up Next looks ahead",
        help = "How many days ahead the Up Next widget looks for events.",
        section = "Calendar",
        range = "1..=31",
        unit = "days",
        advanced
    )]
    pub up_next_days: Count,
    #[settings(
        label = "Up Next shows at most",
        help = "How many events the Up Next widget lists.",
        section = "Calendar",
        range = "1..=10",
        advanced
    )]
    pub up_next_max: Count,
}

impl Default for CalendarSettings {
    fn default() -> Self {
        CalendarSettings {
            sources: "auto".to_owned(),
            first_weekday: FirstWeekday::Monday,
            week_numbers: WeekNumbers::Hide,
            up_next_days: Count(7),
            up_next_max: Count(3),
        }
    }
}

impl Section for CalendarSettings {
    const KEY: &'static str = "calendar";

    fn of(file: &ShellFile) -> &Self {
        &file.calendar
    }
}
