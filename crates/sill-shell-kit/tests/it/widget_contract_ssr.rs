//! The widget contract as markup (design/23-WIDGETS.md section 9): quire's own widgets drawn
//! only through `WidgetCard`, each size and state they take, with the card quire owns around
//! them. Each golden is `tests/snapshots/widget-contract/<name>.html`; each lints clean and
//! every `ds-` class in it is styled by the stylesheet. The timeline's clock behaviour is
//! proven on the virtual clock in `ds-conformance/tests/widget_timeline.rs`.
//!
//! `DS_BLESS=1 cargo test -p ds --features lint --test it widget_contract_ssr` rewrites the goldens.

use crate::support::golden;

use dioxus::prelude::*;
use ds::assembly::ds::Inject;
use ds::components::content::status::battery_state::{BatteryPower, BatteryState, LowAt};
use ds::components::overlays::sheet_attach::Attach;
use ds::components::overlays::sheet_width::SheetWidth;
use ds::prelude::*;
use ds::root::common::Common;
use ds::style::tokens::label_hue::LabelHue;
use ds_lint::{LintConfig, markup};
use sill_shell_kit::battery::device_glyph::Device;
use sill_shell_kit::clock::kind::{ClockTime, DayPhase, Seconds};
use sill_shell_kit::month_grid::data::{
    DayKey, DayMark, DayPlace, Eventful, IsoWeek, MonthDay, MonthGridData, MonthKey, MonthWeek,
    WeekNumbers,
};
use sill_shell_kit::prelude::*;
use sill_shell_kit::tokens::widgets::WidgetMetrics;
use sill_shell_kit::widget::battery::{BatteryCell, BatteryEntry, BatteryWidget};
use sill_shell_kit::widget::calendar::{EventLine, MonthFace, TodayLine};
use sill_shell_kit::widget::calendar::{MonthEntry, MonthWidget};
use sill_shell_kit::widget::clock::{ClockCity, ClockEntry, WorldClockWidget};
use sill_shell_kit::widget::kind::Lift;
use sill_shell_kit::widget::timeline::Dated;
use sill_shell_kit::widget::timeline::EntryDate;
use sill_shell_kit::widget::timeline::Refresh;
use sill_shell_kit::widget::timeline::Timeline;
use sill_shell_kit::widget::{WidgetEdit, WidgetLayout};
use sill_shell_kit::widget::{WireRefresh, WireTimeline};
use std::time::{Duration, Instant};

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

fn cell(name: &str, device: Device, level: u16, power: BatteryPower) -> BatteryCell {
    BatteryCell {
        name: name.to_owned(),
        device,
        state: BatteryState {
            level: Fraction(level),
            power,
            low_at: LowAt::default(),
        },
    }
}

fn four() -> BatteryEntry {
    BatteryEntry::Devices(vec![
        cell("Phone", Device::Phone, 830, BatteryPower::Battery),
        cell("Watch", Device::Watch, 130, BatteryPower::Battery),
        cell("Earbuds", Device::Earbuds, 990, BatteryPower::Charging),
        cell("Keyboard", Device::Keyboard, 960, BatteryPower::Battery),
    ])
}

fn city(name: &str, hour: u8, phase: DayPhase) -> ClockCity {
    ClockCity {
        name: name.to_owned(),
        time: ClockTime {
            hour,
            minute: 25,
            second: Seconds::Hidden,
        },
        phase,
        notes: vec!["Today".to_owned(), "+1HRS".to_owned()],
    }
}

fn cities() -> ClockEntry {
    ClockEntry::Cities(vec![
        city("Taipei", 10, DayPhase::Day),
        city("Tokyo", 11, DayPhase::Day),
        city("London", 3, DayPhase::Night),
    ])
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
    ("battery-solo", || {
        let entry = BatteryEntry::Devices(vec![cell(
            "MacBook",
            Device::Laptop,
            930,
            BatteryPower::Battery,
        )]);
        desktop(
            Theme::Light,
            rsx! { WidgetCard { widget: BatteryWidget, timeline: Timeline::now(entry), size: WidgetSize::Small, common: Common { id: Some("widget-battery".to_string()), ..Common::default() } } },
        )
    }),
    ("battery-grid", || {
        let entry = BatteryEntry::Devices(vec![
            cell("MacBook", Device::Laptop, 930, BatteryPower::Battery),
            cell("Headphones", Device::Headphones, 800, BatteryPower::Battery),
        ]);
        desktop(
            Theme::Dark,
            rsx! { WidgetCard { widget: BatteryWidget, timeline: Timeline::now(entry), size: WidgetSize::Small } },
        )
    }),
    ("battery-row", || {
        desktop(
            Theme::Light,
            rsx! { WidgetCard { widget: BatteryWidget, timeline: Timeline::now(four()), size: WidgetSize::Medium } },
        )
    }),
    ("battery-row-one", || {
        let entry = BatteryEntry::Devices(vec![cell(
            "MacBook",
            Device::Laptop,
            930,
            BatteryPower::Battery,
        )]);
        desktop(
            Theme::Light,
            rsx! { WidgetCard { widget: BatteryWidget, timeline: Timeline::now(entry), size: WidgetSize::Medium } },
        )
    }),
    ("battery-row-two", || {
        let entry = BatteryEntry::Devices(vec![
            cell("MacBook", Device::Laptop, 930, BatteryPower::Battery),
            cell("Headphones", Device::Headphones, 800, BatteryPower::Battery),
        ]);
        desktop(
            Theme::Light,
            rsx! { WidgetCard { widget: BatteryWidget, timeline: Timeline::now(entry), size: WidgetSize::Medium } },
        )
    }),
    ("battery-row-waiting", || {
        desktop(
            Theme::Light,
            rsx! { WidgetCard { widget: BatteryWidget, size: WidgetSize::Medium } },
        )
    }),
    ("battery-row-tile", || {
        center(
            rsx! { WidgetCard { widget: BatteryWidget, timeline: Timeline::now(four()), size: WidgetSize::Medium, host: WidgetHost::Tile } },
        )
    }),
    ("battery-waiting", || {
        desktop(
            Theme::Light,
            rsx! { WidgetCard { widget: BatteryWidget, size: WidgetSize::Small } },
        )
    }),
    ("battery-absent", || {
        let entry = BatteryEntry::Absent("No batteries".to_owned());
        desktop(
            Theme::Light,
            rsx! { WidgetCard { widget: BatteryWidget, timeline: Timeline::now(entry), size: WidgetSize::Small } },
        )
    }),
    ("clock-medium", || {
        desktop(
            Theme::Light,
            rsx! { WidgetCard { widget: WorldClockWidget, timeline: Timeline::now(cities()), size: WidgetSize::Medium } },
        )
    }),
    ("clock-medium-dark", || {
        desktop(
            Theme::Dark,
            rsx! { WidgetCard { widget: WorldClockWidget, timeline: Timeline::now(cities()), size: WidgetSize::Medium } },
        )
    }),
    ("clock-small", || {
        desktop(
            Theme::Light,
            rsx! { WidgetCard { widget: WorldClockWidget, timeline: Timeline::now(cities()), size: WidgetSize::Small } },
        )
    }),
    ("clock-tile", || {
        center(
            rsx! { WidgetCard { widget: WorldClockWidget, timeline: Timeline::now(cities()), size: WidgetSize::Medium, host: WidgetHost::Tile } },
        )
    }),
    ("clock-waiting", || {
        desktop(
            Theme::Light,
            rsx! { WidgetCard { widget: WorldClockWidget, size: WidgetSize::Medium } },
        )
    }),
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
    ("battery-lifted", || {
        desktop(
            Theme::Light,
            rsx! { WidgetCard { widget: BatteryWidget, timeline: Timeline::now(four()), size: WidgetSize::Medium, lift: Lift::Lifted } },
        )
    }),
    ("slot-guide", || {
        desktop(
            Theme::Light,
            rsx! { WidgetSlotGuide { size: WidgetSize::Medium } },
        )
    }),
    ("gallery", || {
        let layout = sill_shell_kit::widget::apply(
            WidgetLayout::default(),
            WidgetEdit::Add {
                kind: BatteryWidget::kind(),
                size: WidgetSize::Small,
                host: WidgetHost::Desktop,
            },
            sill_shell_kit::widget::DesktopGrid {
                columns: 4,
                rows: 3,
            },
        )
        .unwrap_or_default();
        desktop(
            Theme::Light,
            rsx! { WidgetGallery { layout, onedit: |_| {} } },
        )
    }),
    ("edit-widgets-sheet", || {
        rsx! {
            Ds { appearance: Appearance { motion: Motion::Reduced, ..Appearance::default() }, material: Material::Sheet, extent: RootExtent::Viewport, stylesheet: Inject::Host,
                Sheet { label: "Edit Widgets", onclose: |_| {}, attach: Attach::Bottom, width: SheetWidth::Wide,
                    div { style: "{WidgetMetrics::default().style_attr()};height:330px",
                        WidgetGallery { layout: WidgetLayout::default(), onedit: |_| {} }
                    }
                }
            }
        }
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

#[test]
fn every_card_matches_its_golden() {
    let failures: Vec<String> = CASES
        .iter()
        .filter_map(|(name, make)| {
            golden::check(&format!("widget-contract/{name}.html"), &render(*make)).err()
        })
        .collect();
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn every_card_lints_clean_and_every_class_is_styled() {
    let sheet = sill_shell_kit::stylesheet();
    let mut failures = Vec::new();
    for (name, make) in CASES {
        let html = render(*make);
        for offence in markup(&html, sheet, &LintConfig::new(&sill_shell_kit::kits())) {
            failures.push(format!("{name}: {:?} {}", offence.rule, offence.text));
        }
        for class in html
            .split("class=\"")
            .skip(1)
            .filter_map(|rest| rest.split('"').next())
            .flat_map(str::split_whitespace)
            .filter(|class| class.starts_with("ds-"))
        {
            let needle = format!(".{class}");
            let styled = sheet.match_indices(&needle).any(|(at, _)| {
                !sheet[at + needle.len()..]
                    .starts_with(|c: char| c.is_ascii_alphanumeric() || c == '-' || c == '_')
            });
            if !styled {
                failures.push(format!("{name}: .{class} is not styled"));
            }
        }
    }
    failures.dedup();
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// The card is quire's: it says the widget's kind, takes the Space's tint on the desktop and
/// none in the tile, and the widget draws only its content inside.
#[test]
fn the_card_is_quires_and_names_its_widget() {
    let solo = html("battery-solo");
    assert!(solo.contains("data-widget=\"quire.battery\""), "{solo}");
    assert!(solo.contains("data-tint=\"space\""), "{solo}");
    assert!(solo.contains("class=\"ds-frame\""), "{solo}");
    assert!(solo.contains("id=\"widget-battery\""), "{solo}");
    assert!(
        !solo.contains("ds-widget-title"),
        "no header row (M25): {solo}"
    );
    let tile = html("battery-row-tile");
    assert!(tile.contains("data-widget=\"quire.battery\""), "{tile}");
    assert!(!tile.contains("data-tint"), "{tile}");
    assert!(html("clock-medium").contains("data-widget=\"quire.world-clock\""));
    assert!(html("month-small").contains("data-widget=\"quire.month\""));
}

/// Each battery layout, the filled glyph in every ring, the percentage only where the
/// reference shows one, and the honest placeholder: bare tracks, no numbers.
#[test]
fn the_batteries_lay_out_as_the_reference() {
    let solo = html("battery-solo");
    assert!(solo.contains("data-layout=\"solo\""), "{solo}");
    assert!(solo.contains("ds-batteries-hero"), "{solo}");
    assert!(solo.contains(">93%<"), "{solo}");
    assert!(solo.contains("data-device=\"laptop\""), "{solo}");
    let grid = html("battery-grid");
    assert!(grid.contains("data-layout=\"grid\""), "{grid}");
    assert_eq!(
        grid.matches("class=\"ds-battery\"").count(),
        4,
        "two rings, two places"
    );
    assert!(
        !grid.contains("ds-battery-figure"),
        "no numbers on the grid: {grid}"
    );
    let row = html("battery-row");
    assert!(row.contains("data-layout=\"row\""), "{row}");
    assert_eq!(row.matches("ds-batteries-cell").count(), 4, "{row}");
    assert!(
        row.contains("data-tone=\"low\""),
        "the watch at 13 %: {row}"
    );
    assert!(row.contains("data-power=\"charging\""), "{row}");
    for (name, batteries) in [
        ("battery-row-one", 1),
        ("battery-row-two", 2),
        ("battery-row", 4),
    ] {
        let row = html(name);
        assert_eq!(
            row.matches("class=\"ds-batteries-cell\"").count(),
            4,
            "{name}: four places at the fixed pitch (M15): {row}"
        );
        assert_eq!(
            row.matches("data-place=\"empty\"").count(),
            4 - batteries,
            "{name}: a bare track in each place left over: {row}"
        );
        assert_eq!(row.matches('%').count(), batteries, "{name}: {row}");
    }
    let row_waiting = html("battery-row-waiting");
    assert_eq!(row_waiting.matches("data-place=\"empty\"").count(), 4);
    let sheet = sill_shell_kit::stylesheet();
    assert!(
        sheet.contains(".ds-batteries[*|data-layout=row]{ justify-content:center;")
            && !sheet.contains(".ds-batteries[*|data-layout=row]{ justify-content:space-between"),
        "the row keeps its pitch rather than spreading to the ends"
    );
    let waiting = html("battery-waiting");
    assert!(waiting.contains("data-waiting"), "{waiting}");
    assert!(!waiting.contains("ds-battery-arc"), "{waiting}");
    assert!(!waiting.contains('%'), "no made-up level: {waiting}");
    assert!(html("battery-absent").contains(">No batteries<"));
}

/// The clock: a row of dials with their notes on a Medium card, the one large dial alone on a
/// Small one, digits in the tile, and the card following the scheme (settled 2026-09-27).
#[test]
fn the_world_clock_follows_the_size_the_host_and_the_scheme() {
    let medium = html("clock-medium");
    assert_eq!(medium.matches("class=\"ds-clock\"").count(), 3, "{medium}");
    assert_eq!(medium.matches("ds-world-clock-note").count(), 6, "{medium}");
    assert!(medium.contains("data-theme=\"light\""), "{medium}");
    assert!(html("clock-medium-dark").contains("data-theme=\"dark\""));
    let small = html("clock-small");
    assert_eq!(small.matches("class=\"ds-clock\"").count(), 1, "{small}");
    assert!(!small.contains("ds-world-clock-note"), "{small}");
    let tile = html("clock-tile");
    assert!(tile.contains("data-look=\"digital\""), "{tile}");
    assert!(!tile.contains("data-look=\"analog\""), "{tile}");
    let waiting = html("clock-waiting");
    assert!(
        waiting.contains(">10:09<") || waiting.contains(" 10:09\""),
        "{waiting}"
    );
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

/// A size the widget does not draw is held to its first: the battery has no Large.
#[test]
fn a_card_asks_only_for_sizes_its_widget_draws() {
    assert_eq!(
        sill_shell_kit::widget::fit::<BatteryWidget>(WidgetSize::Large),
        WidgetSize::Small
    );
    assert_eq!(
        sill_shell_kit::widget::fit::<BatteryWidget>(WidgetSize::Medium),
        WidgetSize::Medium
    );
    assert_eq!(
        sill_shell_kit::widget::fit::<WorldClockWidget>(WidgetSize::Large),
        WidgetSize::Medium
    );
    let large = render(|| {
        desktop(
            Theme::Light,
            rsx! { WidgetCard { widget: BatteryWidget, timeline: Timeline::now(four()), size: WidgetSize::Large } },
        )
    });
    assert!(large.contains("data-size=\"small\""), "{large}");
}

/// Every registered kind is unique and draws its placeholder on quire's card.
#[test]
fn the_registry_previews_each_widget() {
    let registry = WidgetRegistry::quire();
    assert_eq!(registry.all().len(), 3);
    for info in registry.all() {
        assert!(!info.sizes.is_empty(), "{:?}", info.kind);
    }
    let previews = render(|| {
        let registry = WidgetRegistry::quire();
        desktop(
            Theme::Light,
            rsx! {
                for info in registry.all().iter() {
                    {info.preview(info.sizes[0], WidgetHost::Desktop, Lift::Rest)}
                }
            },
        )
    });
    for kind in ["quire.battery", "quire.world-clock", "quire.month"] {
        assert!(
            previews.contains(&format!("data-widget=\"{kind}\"")),
            "{kind}: {previews}"
        );
    }
    assert_eq!(BatteryWidget::kind().as_str(), "quire.battery");
}

/// A timeline crosses a process as JSON with its dates as offsets from sending, and comes back
/// dated from its arrival (design/23 section 9.5).
#[test]
fn a_timeline_crosses_the_wire_as_offsets() {
    let sent = Instant::now();
    let timeline = Timeline::new(
        vec![
            Dated::new(EntryDate::Start, four()),
            Dated::new(
                EntryDate::At(sent + Duration::from_secs(60)),
                BatteryEntry::Waiting,
            ),
        ],
        Refresh::After(sent + Duration::from_secs(300)),
    );
    let wire = WireTimeline::sent(timeline.clone(), sent);
    assert_eq!(wire.refresh, WireRefresh::AfterMs(300_000));
    let json = serde_json::to_string(&wire).expect("serialises");
    assert!(json.contains("\"after_ms\":60000"), "{json}");
    assert!(json.contains("\"device\":\"earbuds\""), "{json}");
    let back: WireTimeline<BatteryEntry> = serde_json::from_str(&json).expect("parses");
    let arrived = sent + Duration::from_millis(250);
    let received = back.received(arrived);
    assert_eq!(received.current(arrived), Some(&four()));
    assert_eq!(
        received.current(arrived + Duration::from_secs(60)),
        Some(&BatteryEntry::Waiting)
    );
    assert_eq!(
        received.refresh(),
        Refresh::After(arrived + Duration::from_secs(300))
    );
}

/// A card picked up says so, and the stylesheet's lifted rule restates the transparent root's
/// card selector, so `--shadow-drag` replaces the resting drop instead of stacking under it
///; the slot guide is the footprint of its size, hidden from assistive technology.
#[test]
fn a_lifted_card_swaps_its_shadow_and_the_guide_is_a_footprint() {
    let lifted = html("battery-lifted");
    assert!(lifted.contains("data-lift=\"lifted\""), "{lifted}");
    assert!(
        !html("battery-row").contains("data-lift"),
        "a card at rest writes nothing"
    );
    let sheet = sill_shell_kit::stylesheet();
    assert!(
        sheet.contains(".ds[*|data-material][*|data-chrome=transparent] .ds-widget[*|data-host=desktop][*|data-lift=lifted]{ box-shadow:var(--shadow-drag); }"),
        "the lifted shadow wins over the card's --m-box"
    );

    let guide = html("slot-guide");
    assert!(guide.contains("class=\"ds-widget-slot a-fade\""), "{guide}");
    assert!(guide.contains("data-size=\"medium\""), "{guide}");
}

/// The gallery lists every registered widget with its description, draws the first at each of
/// its sizes from its preview entry, and lists what is placed per surface.
#[test]
fn the_gallery_browses_the_registry_and_lists_the_layout() {
    let gallery = html("gallery");
    assert_eq!(
        gallery.matches("class=\"ds-list-item\"").count(),
        3,
        "one list row per registered widget: {gallery}"
    );
    assert!(
        gallery.contains(">See the charge of this computer and your devices.<"),
        "{gallery}"
    );
    assert!(
        gallery.contains("class=\"ds-widget-gallery-preview\" data-size=\"small\""),
        "the batteries at their one desktop size: {gallery}"
    );
    assert!(
        gallery.contains(">82%<") || gallery.contains("aria-valuenow=\"82\""),
        "the preview entry: {gallery}"
    );
    assert_eq!(
        gallery.matches("class=\"ds-widget-gallery-row\"").count(),
        1,
        "{gallery}"
    );
    assert!(gallery.contains(">Batteries<"), "{gallery}");
}

/// Edit Widgets draws each widget once, at the one size it takes: one preview, no
/// size control on it or on a placed row, and every class it writes styled.
#[test]
fn the_gallery_offers_one_size_per_widget() {
    let gallery = html("gallery");
    assert_eq!(
        gallery
            .matches("class=\"ds-widget-gallery-preview\"")
            .count(),
        1,
        "{gallery}"
    );
    assert_eq!(
        gallery.matches("class=\"ds-widget\"").count(),
        1,
        "one card: {gallery}"
    );
    assert!(!gallery.contains("ds-segmented"), "{gallery}");
    assert!(!gallery.contains("data-lift=\"lifted\""), "{gallery}");
    assert!(gallery.contains(">Batteries<"), "the placed row: {gallery}");
}

/// Edit Widgets' sheet at the bottom edge: the sheet says its attachment and its width; the
/// stylesheet holds it under half the root and slides it in from below.
#[test]
fn the_bottom_sheet_says_its_attachment_and_its_width() {
    let sheet = html("edit-widgets-sheet");
    assert!(sheet.contains("data-attach=\"bottom\""), "{sheet}");
    assert!(sheet.contains("data-width=\"wide\""), "{sheet}");
    assert!(sheet.contains("height:330px"), "{sheet}");
    assert!(sheet.contains("ds-widget-gallery"), "{sheet}");
    let css = sill_shell_kit::stylesheet();
    assert!(
        css.contains("max-height:calc(50% - var(--s-8))"),
        "never taller than half the root"
    );
    assert!(css.contains("--sheet-dy:calc(100% + var(--s-36))"));
}
