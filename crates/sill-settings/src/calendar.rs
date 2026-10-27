//! `calendar.*`: where the calendar widgets read events, and how the month is drawn (design/20
//! §1.12, which sill moves into the notification center's widgets as macOS 11 does). All Advanced.

use ds_core::word::Word;
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
        help = "Auto finds your calendars; off uses none; or list .ics files and folders, separated by ;.",
        section = "Calendar",
        advanced
    )]
    pub sources: String,
    #[settings(
        label = "First day of the week",
        help = "Which day starts each week.",
        section = "Calendar",
        advanced
    )]
    pub first_weekday: FirstWeekday,
    #[settings(
        label = "Week numbers",
        help = "Shows week numbers beside the month.",
        section = "Calendar",
        advanced
    )]
    pub week_numbers: WeekNumbers,
}

impl Default for CalendarSettings {
    fn default() -> Self {
        CalendarSettings {
            sources: "auto".to_owned(),
            first_weekday: FirstWeekday::Monday,
            week_numbers: WeekNumbers::Hide,
        }
    }
}
