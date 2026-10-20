//! The widget contract's timeline on a real Blitz document and the virtual clock
//! (design/23-WIDGETS.md section 9.2): a card shows each entry from its date on, its placeholder
//! before the first, asks its provider for a new timeline once when the policy comes due (never
//! sooner than the floor), follows a new timeline the moment it is handed one, and between
//! dates asks for no frame at all (the idle-frame rule). Every test runs on `Clock::Virtual`, so
//! the dates are exact whatever the machine's load.

use dioxus::prelude::*;
use ds::prelude::*;
use ds_harness::harness::assert_settles_to_zero_frames;
use ds_harness::{Clock, Harness, HarnessConfig, Viewport};
use sill_shell_kit::prelude::*;
use sill_shell_kit::widget::REFRESH_FLOOR;
use sill_shell_kit::widget::contract::{NoIntent, Widget, WidgetContext, WidgetKind};
use sill_shell_kit::widget::timeline::{Dated, EntryDate, Refresh, RefreshAsk, Timeline};
use std::cell::RefCell;
use std::time::Duration;

const VIEW: Viewport = Viewport {
    width: 400,
    height: 200,
    scale_percent: 100,
};

thread_local! {
    /// What the card asked its provider, in order.
    static ASKS: RefCell<Vec<RefreshAsk>> = const { RefCell::new(Vec::new()) };
}

/// A new timeline the test hands the card, or none (the mounted one stays).
static NEXT: GlobalSignal<Option<Timeline<String>>> = Signal::global(|| None);

/// A widget that shows its entry as text: the timeline is what is under test, not a face.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
struct Stamp;

impl Widget for Stamp {
    type Entry = String;
    type Intent = NoIntent;

    fn kind() -> WidgetKind {
        WidgetKind::fixed("test.stamp")
    }

    fn sizes() -> &'static [WidgetSize] {
        &[WidgetSize::Small]
    }

    fn placeholder(_size: WidgetSize) -> String {
        "--".to_owned()
    }

    fn view(entry: &String, _cx: WidgetContext<NoIntent>) -> Element {
        rsx! { span { class: "stamp", "{entry}" } }
    }
}

fn taipei(hour: u8, minute: u8) -> String {
    format!("{hour:02}:{minute:02}")
}

fn secs(n: u64) -> Duration {
    Duration::from_secs(n)
}

/// `timeline` built on the clock at mount, on a card whose refresh asks are recorded.
fn stage(timeline: fn() -> Timeline<String>) -> Element {
    let mounted = use_hook(timeline);
    let timeline = NEXT().unwrap_or(mounted);
    rsx! {
        Ds { sheet: Some(sill_shell_kit::stylesheet()), appearance: Appearance { motion: Motion::Reduced, ..Appearance::default() }, material: Material::Widget, chrome: Some(RootChrome::Transparent),
            WidgetCard { widget: Stamp, timeline, size: WidgetSize::Small,
                onrefresh: |ask| ASKS.with(|asks| asks.borrow_mut().push(ask)) }
        }
    }
}

/// 10:00 now, 10:01 a minute on, 10:02 two minutes on; ask again at the end.
fn three_minutes() -> Timeline<String> {
    let now = ds::base::time::clock::now();
    Timeline::new(
        vec![
            Dated::new(EntryDate::Start, taipei(10, 0)),
            Dated::new(EntryDate::At(now + secs(60)), taipei(10, 1)),
            Dated::new(EntryDate::At(now + secs(120)), taipei(10, 2)),
        ],
        Refresh::AtEnd,
    )
}

/// The first entry half a minute away.
fn later() -> Timeline<String> {
    let now = ds::base::time::clock::now();
    Timeline::new(
        vec![Dated::new(EntryDate::At(now + secs(30)), taipei(8, 30))],
        Refresh::Never,
    )
}

/// One entry, and a refresh already due.
fn overdue() -> Timeline<String> {
    let now = ds::base::time::clock::now();
    Timeline::new(
        vec![Dated::new(EntryDate::Start, taipei(9, 0))],
        Refresh::After(now),
    )
}

#[allow(non_snake_case)]
fn ThreeMinutes() -> Element {
    stage(three_minutes)
}

#[allow(non_snake_case)]
fn Later() -> Element {
    stage(later)
}

#[allow(non_snake_case)]
fn Overdue() -> Element {
    stage(overdue)
}

fn harness(app: fn() -> Element) -> Harness {
    ASKS.with(|asks| asks.borrow_mut().clear());
    Harness::new(app, HarnessConfig::new(VIEW).with_clock(Clock::Virtual))
}

fn shown(harness: &Harness) -> String {
    harness.text_of(".stamp").unwrap_or_default()
}

fn asks() -> Vec<RefreshAsk> {
    ASKS.with(|asks| asks.borrow().clone())
}

#[test]
fn a_card_shows_each_entry_from_its_date_and_asks_once_at_the_end() {
    let mut harness = harness(ThreeMinutes);
    assert_eq!(shown(&harness), "10:00");
    harness.advance(secs(59));
    assert_eq!(shown(&harness), "10:00", "a second before its date");
    harness.advance(secs(1));
    assert_eq!(shown(&harness), "10:01", "on its date");
    assert!(asks().is_empty());
    harness.advance(secs(60));
    assert_eq!(shown(&harness), "10:02");
    assert_eq!(
        asks(),
        [RefreshAsk::Ended],
        "asked when the last entry showed"
    );
    harness.advance(secs(600));
    assert_eq!(
        asks(),
        [RefreshAsk::Ended],
        "asked once, then it waits for the answer"
    );
    assert_eq!(shown(&harness), "10:02", "the last entry stays");
    assert_settles_to_zero_frames(&mut harness);
}

#[test]
fn a_card_shows_its_placeholder_before_the_first_entry() {
    let mut harness = harness(Later);
    assert_eq!(shown(&harness), "--", "the placeholder: {}", harness.html());
    harness.advance(secs(29));
    assert_eq!(shown(&harness), "--");
    harness.advance(secs(1));
    assert_eq!(shown(&harness), "08:30");
    assert_settles_to_zero_frames(&mut harness);
    assert!(asks().is_empty(), "Never asks nothing");
}

#[test]
fn a_refresh_already_due_waits_for_the_floor() {
    let mut harness = harness(Overdue);
    harness.advance(REFRESH_FLOOR - Duration::from_millis(1));
    assert!(asks().is_empty(), "not before the floor");
    harness.advance(Duration::from_millis(1));
    assert_eq!(asks(), [RefreshAsk::Due]);
    assert_settles_to_zero_frames(&mut harness);
}

#[test]
fn a_new_timeline_is_followed_at_once_and_the_old_one_dropped() {
    let mut harness = harness(ThreeMinutes);
    harness.advance(secs(30));
    let answer = harness.within(|| {
        let now = ds::base::time::clock::now();
        Timeline::new(
            vec![
                Dated::new(EntryDate::Start, taipei(11, 0)),
                Dated::new(EntryDate::At(now + secs(45)), taipei(11, 1)),
            ],
            Refresh::Never,
        )
    });
    harness.within(|| *NEXT.write() = Some(answer));
    harness.advance(Duration::ZERO);
    assert_eq!(shown(&harness), "11:00", "the new timeline at once");
    harness.advance(secs(30));
    assert_eq!(
        shown(&harness),
        "11:00",
        "the old timeline's 10:01 no longer lands"
    );
    harness.advance(secs(15));
    assert_eq!(shown(&harness), "11:01");
    harness.advance(secs(600));
    assert!(asks().is_empty(), "the dropped timeline's AtEnd never asks");
    assert_settles_to_zero_frames(&mut harness);
}
