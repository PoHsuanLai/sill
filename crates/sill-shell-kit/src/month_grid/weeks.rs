//! A MonthGrid's weeks: the rows of day cells, mounted under the month's key so a new month is a
//! new body, which plays its slide once as it mounts and drops the class at the slide's settle
//! (design/04-COMPONENTS.md section 39; design/05-MOTION.md section 7, timers instead of
//! `animationend`). A render that keeps the month keeps the body, and plays nothing.

use crate::month_grid::data::{DayKey, Eventful, MonthDay, MonthKey, MonthWeek, WeekNumbers};
use dioxus::prelude::*;
use ds::base::word::Word;
use ds::motion::anim::Anim;
use ds::motion::timer::{TimerPhase, use_motion_timer};
use std::cmp::Ordering;

/// How a month's weeks arrive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum MonthSlide {
    /// The first month drawn: no motion.
    Still,
    /// An earlier month: `slide-l`.
    FromLeft,
    /// A later month: `slide-r`.
    FromRight,
}

impl MonthSlide {
    /// The slide from `last` to `now`, or `None` when the month is the same.
    pub(crate) fn between(last: MonthKey, now: MonthKey) -> Option<MonthSlide> {
        match now.cmp(&last) {
            Ordering::Equal => None,
            Ordering::Greater => Some(MonthSlide::FromRight),
            Ordering::Less => Some(MonthSlide::FromLeft),
        }
    }

    /// The animation it plays, if any.
    fn anim(self) -> Option<Anim> {
        match self {
            MonthSlide::Still => None,
            MonthSlide::FromLeft => Some(Anim::SlideL),
            MonthSlide::FromRight => Some(Anim::SlideR),
        }
    }
}

/// The weeks of one month. Mount it under the month's key: its slide starts as it mounts.
#[component]
pub(crate) fn MonthWeeks(
    weeks: Vec<MonthWeek>,
    numbers: WeekNumbers,
    slide: MonthSlide,
    onpick: Option<EventHandler<DayKey>>,
) -> Element {
    let playing = use_slide(slide);
    let class = match playing {
        Some(anim) => format!("ds-month-weeks {}", anim.class()),
        None => "ds-month-weeks".to_owned(),
    };
    let pulse = playing.map(|_| "a");
    rsx! {
        div { class, "data-pulse": pulse,
            for week in weeks {
                div { key: "{week.number.0}", class: "ds-month-row",
                    if numbers == WeekNumbers::Show {
                        span { class: "ds-month-week", "{week.number.0}" }
                    }
                    for day in week.days {
                        {cell(day, onpick)}
                    }
                }
            }
        }
    }
}

/// The animation playing now: the slide from its mount until it settles, then none.
fn use_slide(slide: MonthSlide) -> Option<Anim> {
    let timer = use_motion_timer(slide.anim().unwrap_or(Anim::SlideR));
    use_hook(|| {
        if slide.anim().is_some() {
            timer.start(EventHandler::new(|()| {}));
        }
    });
    match timer.phase() {
        TimerPhase::Running => slide.anim(),
        TimerPhase::Idle | TimerPhase::Settled => None,
    }
}

/// One day: a `button` when it can be picked, else a `span`.
fn cell(day: MonthDay, onpick: Option<EventHandler<DayKey>>) -> Element {
    let key = format!("{}-{}", day.key.month, day.key.day);
    let dot = day.events == Eventful::Busy;
    let place = day.place.slug();
    let events = day.events.slug();
    let current = day.mark.aria_current();
    let label = day.key.day;
    match onpick {
        Some(onpick) => rsx! {
            button {
                key: "{key}",
                r#type: "button",
                class: "ds-month-day",
                "data-kind": "pressable",
                "data-place": place,
                "data-events": events,
                "aria-current": current,
                onclick: move |_| onpick.call(day.key),
                span { class: "ds-month-num", "{label}" }
                if dot {
                    span { class: "ds-month-dot" }
                }
            }
        },
        None => rsx! {
            span {
                key: "{key}",
                class: "ds-month-day",
                "data-place": place,
                "data-events": events,
                "aria-current": current,
                span { class: "ds-month-num", "{label}" }
                if dot {
                    span { class: "ds-month-dot" }
                }
            }
        },
    }
}

#[cfg(test)]
mod tests {
    use super::MonthSlide;
    use crate::month_grid::data::MonthKey;
    use ds::motion::anim::Anim;

    #[test]
    fn a_change_of_month_slides_by_their_order() {
        let sept = MonthKey {
            year: 2026,
            month: 9,
        };
        let cases = [
            (
                sept,
                MonthKey { month: 10, ..sept },
                Some(MonthSlide::FromRight),
            ),
            (
                sept,
                MonthKey { month: 8, ..sept },
                Some(MonthSlide::FromLeft),
            ),
            (
                MonthKey {
                    year: 2026,
                    month: 12,
                },
                MonthKey {
                    year: 2027,
                    month: 1,
                },
                Some(MonthSlide::FromRight),
            ),
            (sept, sept, None),
        ];
        for (last, now, want) in cases {
            assert_eq!(MonthSlide::between(last, now), want, "{last:?} to {now:?}");
        }
        assert_eq!(MonthSlide::FromRight.anim(), Some(Anim::SlideR));
        assert_eq!(MonthSlide::FromLeft.anim(), Some(Anim::SlideL));
        assert_eq!(MonthSlide::Still.anim(), None);
    }
}
