//! The World Clock widget on the widget contract (design/23-WIDGETS.md sections 4.2 and 9.6):
//! Small, one large dial (the city dropped, as the reference's); Medium, up to four dials with
//! the city and the provider's notes (the day, the offset) under each. On the desktop the dials
//! are analog; in the notification center's tile the time is digits. The card follows the
//! scheme (settled 2026-09-27): a light card in the light scheme, a dark one in the dark.

use crate::clock::face::ClockFace;
use crate::clock::kind::{ClockLook, ClockTime, DayPhase, Seconds};
use crate::widget::contract::{NoIntent, Widget, WidgetContext, WidgetKind};
use crate::widget::kind::{WidgetHost, WidgetSize};
use dioxus::prelude::*;
use ds::components::content::text_runs::TextLine;
use serde::{Deserialize, Serialize};

/// The World Clock widget.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct WorldClockWidget;

/// One city.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ClockCity {
    /// Its name.
    pub name: String,
    /// The time there.
    pub time: ClockTime,
    /// Day or night there.
    pub phase: DayPhase,
    /// The quiet lines under the city, in the provider's words: its day and its offset.
    pub notes: Vec<String>,
}

/// One moment of the World Clock widget.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClockEntry {
    /// No cities read yet: blank day dials.
    #[default]
    Waiting,
    /// The cities; the first four are drawn (one on a Small card).
    Cities(Vec<ClockCity>),
    /// No city is set: a quiet line in the provider's words.
    Absent(String),
}

/// How many cities a Medium card holds.
pub const MAX_CITIES: usize = 4;

/// How many cities `size` shows.
pub(crate) fn room(size: WidgetSize) -> usize {
    match size {
        WidgetSize::Small => 1,
        WidgetSize::Medium | WidgetSize::Large => MAX_CITIES,
    }
}

/// Dials on the desktop, digits in the notification center's tile.
pub(crate) fn look_of(host: WidgetHost) -> ClockLook {
    match host {
        WidgetHost::Desktop => ClockLook::Analog,
        WidgetHost::Tile => ClockLook::Digital,
    }
}

/// The time a waiting dial stands at.
const WAITING: ClockTime = ClockTime {
    hour: 10,
    minute: 9,
    second: Seconds::Hidden,
};

impl Widget for WorldClockWidget {
    type Entry = ClockEntry;
    type Intent = NoIntent;

    fn kind() -> WidgetKind {
        WidgetKind::fixed("quire.world-clock")
    }

    fn name() -> TextLine {
        TextLine::from("World Clock")
    }

    fn sizes() -> &'static [WidgetSize] {
        &[WidgetSize::Medium, WidgetSize::Small]
    }

    fn description() -> TextLine {
        TextLine::from("See the time in cities around the world.")
    }

    fn placeholder(_size: WidgetSize) -> ClockEntry {
        ClockEntry::Waiting
    }

    fn preview(_size: WidgetSize) -> ClockEntry {
        let city = |name: &str, hour, phase| ClockCity {
            name: name.to_owned(),
            time: ClockTime {
                hour,
                minute: 9,
                second: Seconds::Hidden,
            },
            phase,
            notes: Vec::new(),
        };
        ClockEntry::Cities(vec![
            city("Cupertino", 10, DayPhase::Day),
            city("Tokyo", 2, DayPhase::Night),
            city("Sydney", 3, DayPhase::Night),
            city("Paris", 19, DayPhase::Night),
        ])
    }

    fn view(entry: &ClockEntry, cx: WidgetContext<NoIntent>) -> Element {
        let look = look_of(cx.host);
        let count = room(cx.size);
        match entry {
            ClockEntry::Waiting => {
                let blank = ClockCity {
                    name: String::new(),
                    time: WAITING,
                    phase: DayPhase::Day,
                    notes: Vec::new(),
                };
                dials(&vec![blank; count], cx.size, look)
            }
            ClockEntry::Cities(cities) => dials(&cities[..cities.len().min(count)], cx.size, look),
            ClockEntry::Absent(words) => rsx! {
                div { class: "ds-world-clock",
                    span { class: "ds-world-clock-quiet", "{words}" }
                }
            },
        }
    }
}

/// The dials: on a Small card the one face alone, filling it; else a row with the notes.
fn dials(cities: &[ClockCity], size: WidgetSize, look: ClockLook) -> Element {
    match size {
        WidgetSize::Small => rsx! {
            for city in cities {
                ClockFace { time: city.time, phase: city.phase, look, label: city.name.clone() }
            }
        },
        WidgetSize::Medium | WidgetSize::Large => rsx! {
            div { class: "ds-world-clock",
                for (at, city) in cities.iter().enumerate() {
                    div { key: "{at}-{city.name}", class: "ds-world-clock-city",
                        ClockFace { time: city.time, phase: city.phase, look, label: city.name.clone() }
                        for note in city.notes.iter() {
                            span { class: "ds-world-clock-note", "{note}" }
                        }
                    }
                }
            }
        },
    }
}

#[cfg(test)]
mod tests {
    use super::{look_of, room};
    use crate::clock::kind::ClockLook;
    use crate::widget::kind::{WidgetHost, WidgetSize};

    #[test]
    fn a_small_card_holds_one_city_and_the_tile_draws_digits() {
        assert_eq!(room(WidgetSize::Small), 1);
        assert_eq!(room(WidgetSize::Medium), 4);
        assert_eq!(look_of(WidgetHost::Desktop), ClockLook::Analog);
        assert_eq!(look_of(WidgetHost::Tile), ClockLook::Digital);
    }
}
