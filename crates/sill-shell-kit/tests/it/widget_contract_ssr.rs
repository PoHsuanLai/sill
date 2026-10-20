//! The widget contract as markup (design/23-WIDGETS.md section 9): quire's own widgets drawn
//! only through `WidgetCard`, each size and state they take, with the card quire owns around
//! them. Each golden is `tests/snapshots/widget-contract/<name>.html`; each lints clean and
//! every `ds-` class in it is styled by the stylesheet. The timeline's clock behaviour is
//! proven on the virtual clock in `ds-conformance/tests/widget_timeline.rs`.
//!
//! `DS_BLESS=1 cargo test -p ds --features lint --test it widget_contract_ssr` rewrites the goldens.

use crate::support::specimens::Specimen;

use dioxus::prelude::*;
use ds::assembly::ds::Inject;
use ds::prelude::*;
use ds::style::tokens::label_hue::LabelHue;
use sill_shell_kit::battery::device_glyph::Device;
use sill_shell_kit::month_grid::data::{
    DayKey, DayMark, DayPlace, Eventful, IsoWeek, MonthDay, MonthGridData, MonthKey, MonthWeek,
    WeekNumbers,
};
use sill_shell_kit::prelude::*;
use sill_shell_kit::tokens::widgets::WidgetMetrics;
use sill_shell_kit::widget::calendar::{EventLine, MonthFace, TodayLine};
use sill_shell_kit::widget::calendar::{MonthEntry, MonthWidget};
use sill_shell_kit::widget::timeline::Timeline;

#[derive(Props, Clone)]
struct HostProps {
    make: fn() -> Element,
}

impl PartialEq for HostProps {
    fn eq(&self, _: &Self) -> bool {
        false
    }
}

fn host(props: HostProps) -> Element {
    (props.make)()
}

fn render(make: fn() -> Element) -> String {
    let mut dom = VirtualDom::new_with_props(host, HostProps { make });
    dom.rebuild_in_place();
    // Floating surfaces (a sheet) arrive through the overlay host once the effects that register
    // them have run.
    for _ in 0..3 {
        dom.render_immediate(&mut dioxus::core::NoOpMutations);
    }
    dioxus_ssr::render(&dom)
}

/// `body` on the desktop layer under Reduced motion, so a ring's golden is its level at rest.
fn desktop(theme: Theme, body: Element) -> Element {
    rsx! {
        Ds { appearance: Appearance { theme, motion: Motion::Reduced, ..Appearance::default() }, material: Material::Widget, chrome: Some(RootChrome::Transparent), stylesheet: Inject::Host,
            div { style: WidgetMetrics::default().style_attr(), {body} }
        }
    }
}

/// `body` in the notification center: a Popover root.
fn center(body: Element) -> Element {
    rsx! {
        Ds { appearance: Appearance { motion: Motion::Reduced, ..Appearance::default() }, material: Material::Popover, stylesheet: Inject::Host,
            div { style: WidgetMetrics::default().style_attr(), {body} }
        }
    }
}

fn day(day: i8, place: DayPlace, mark: DayMark) -> MonthDay {
    MonthDay {
        key: DayKey {
            year: 2026,
            month: 9,
            day,
        },
        place,
        mark,
        events: Eventful::Free,
    }
}

/// One week of September 2026, the 27th today, no events.
fn september() -> MonthEntry {
    september_with(Vec::new())
}

/// The week of `september` with `events` today and the words for none.
fn september_with(events: Vec<EventLine>) -> MonthEntry {
    let days = [21, 22, 23, 24, 25, 26, 27].map(|n| {
        let mark = if n == 27 {
            DayMark::Today
        } else {
            DayMark::Plain
        };
        day(n, DayPlace::InMonth, mark)
    });
    MonthEntry::Month(Box::new(MonthFace {
        today: Some(TodayLine {
            weekday: "Sunday".to_owned(),
            day: 27,
        }),
        events,
        no_events: "No events today".to_owned(),
        grid: MonthGridData {
            month: MonthKey {
                year: 2026,
                month: 9,
            },
            title: "September".into(),
            heads: ["M", "T", "W", "T", "F", "S", "S"].map(Into::into),
            weeks: vec![MonthWeek {
                number: IsoWeek(39),
                days,
            }],
        },
        weeks: WeekNumbers::Hide,
    }))
}

fn events() -> Vec<EventLine> {
    let event = |time: &str, title: &str, hue| EventLine {
        time: time.to_owned(),
        title: title.to_owned(),
        hue,
    };
    vec![
        event("10:00", "Design review", LabelHue::Blue),
        event("13:30", "Lunch", LabelHue::Green),
        event("17:00", "Climbing", LabelHue::Amber),
        event("20:00", "Call home", LabelHue::Red),
    ]
}

type Case = (&'static str, fn() -> Element);

const CASES: &[Case] = &[
    ("month-small", || {
        desktop(
            Theme::Light,
            rsx! { WidgetCard { widget: MonthWidget, timeline: Timeline::now(september()), size: WidgetSize::Small, onintent: |_| {} } },
        )
    }),
    ("month-medium", || {
        desktop(
            Theme::Light,
            rsx! { WidgetCard { widget: MonthWidget, timeline: Timeline::now(september_with(events())), size: WidgetSize::Medium } },
        )
    }),
    ("month-medium-free", || {
        desktop(
            Theme::Light,
            rsx! { WidgetCard { widget: MonthWidget, timeline: Timeline::now(september()), size: WidgetSize::Medium } },
        )
    }),
    ("month-large", || {
        desktop(
            Theme::Dark,
            rsx! { WidgetCard { widget: MonthWidget, timeline: Timeline::now(september_with(events())), size: WidgetSize::Large } },
        )
    }),
    ("month-large-tile", || {
        center(
            rsx! { WidgetCard { widget: MonthWidget, timeline: Timeline::now(september()), size: WidgetSize::Large, host: WidgetHost::Tile } },
        )
    }),
    ("devices", || {
        desktop(
            Theme::Light,
            rsx! {
                for device in Device::ALL.iter().copied() {
                    DeviceGlyph { device, size: IconSize::Base }
                }
            },
        )
    }),
];

fn html(name: &str) -> String {
    let (_, make) = CASES
        .iter()
        .find(|(case, _)| *case == name)
        .unwrap_or_else(|| panic!("no case {name}"));
    render(*make)
}

pub(crate) fn specimens() -> Vec<Specimen> {
    CASES
        .iter()
        .map(|(name, make)| Specimen::of(&format!("widget-contract/{name}.html"), render(*make)))
        .collect()
}

/// The card is quire's: it says the widget's kind, takes the Space's tint on the desktop and
/// none in the tile, and the widget draws only its content inside.
#[test]
fn the_card_is_quires_and_names_its_widget() {
    let desktop = html("month-small");
    assert!(desktop.contains("data-widget=\"quire.month\""), "{desktop}");
    assert!(desktop.contains("data-tint=\"space\""), "{desktop}");
    assert!(desktop.contains("class=\"ds-frame\""), "{desktop}");
    assert!(
        !desktop.contains("ds-widget-title"),
        "no header row (M25): {desktop}"
    );
    let tile = html("month-large-tile");
    assert!(tile.contains("data-widget=\"quire.month\""), "{tile}");
    assert!(!tile.contains("data-tint"), "{tile}");
}

/// The month follows the reference's layouts (design/23 section 5.2): Small the compact month
/// alone; Medium the today column (weekday, date, the next event or the quiet line) beside the
/// compact month; Large the regular month over at most three of the day's events, or the quiet
/// line; on a desktop card the title is `--ink-soft` (option (b)).
#[test]
fn the_month_takes_the_reference_layout_at_each_size() {
    let medium = html("month-medium");
    assert!(medium.contains("data-layout=\"split\""), "{medium}");
    assert!(medium.contains("data-density=\"compact\""), "{medium}");
    assert!(
        medium.contains(">Sunday<") && medium.contains(">27<"),
        "{medium}"
    );
    assert_eq!(
        medium.matches("class=\"ds-month-event\"").count(),
        1,
        "the next event only"
    );
    assert!(medium.contains(">Design review<"), "{medium}");
    let free = html("month-medium-free");
    assert!(free.contains(">No events today<"), "{free}");
    let large = html("month-large");
    assert!(large.contains("data-layout=\"stack\""), "{large}");
    assert!(large.contains("data-density=\"regular\""), "{large}");
    assert_eq!(
        large.matches("class=\"ds-month-event\"").count(),
        3,
        "{large}"
    );
    assert!(large.contains("data-hue=\"green\""), "{large}");
    assert!(html("month-large-tile").contains(">No events today<"));
    assert!(!html("month-small").contains("ds-month-today"));
    let sheet = sill_shell_kit::stylesheet();
    assert!(
        sheet.contains(".ds-widget[*|data-host=desktop] .ds-month-title,"),
        "option (b)"
    );
}

/// The month's step buttons exist only when the host listens for the widget's intents.
#[test]
fn the_month_steps_only_with_a_listener() {
    let small = html("month-small");
    assert!(small.contains("data-density=\"compact\""), "{small}");
    assert!(small.contains("ds-month-step"), "{small}");
    let tile = html("month-large-tile");
    assert!(tile.contains("data-density=\"regular\""), "{tile}");
    assert!(
        !tile.contains("ds-month-step") && !tile.contains("aria-label=\"Next"),
        "{tile}"
    );
}
