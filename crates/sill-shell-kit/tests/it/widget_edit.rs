//! Moving and choosing widgets on a real Blitz document and the virtual clock (design/23
//! section 9.7-9.8): a card picked up says so and settles to zero frames, lifted
//! and put down; the drop-slot guide fades in and then asks for nothing; the Edit Widgets gallery
//! draws each widget once at the one size it takes, hands the host an edit for each
//! button, and shows the layout the host hands back.

use dioxus::prelude::*;
use ds::prelude::*;
use ds_harness::harness::assert_settles_to_zero_frames;
use ds_harness::{Clock, Harness, HarnessConfig, Input, Viewport};
use sill_shell_kit::prelude::*;
use sill_shell_kit::tokens::widgets::WidgetMetrics;
use sill_shell_kit::widget::battery::BatteryWidget;
use sill_shell_kit::widget::kind::Lift;
use sill_shell_kit::widget::timeline::Timeline;
use sill_shell_kit::widget::{DesktopGrid, WidgetEdit, WidgetLayout, apply};
use std::time::Duration;

const VIEW: Viewport = Viewport {
    width: 1200,
    height: 1000,
    scale_percent: 100,
};

static LIFT: GlobalSignal<Lift> = Signal::global(Lift::default);

const GRID: DesktopGrid = DesktopGrid {
    columns: 4,
    rows: 3,
};

fn desktop(body: Element) -> Element {
    rsx! {
        Ds { sheet: Some(sill_shell_kit::stylesheet()), appearance: Appearance::default(), material: Material::Widget, chrome: Some(RootChrome::Transparent),
            div { style: WidgetMetrics::default().style_attr(), {body} }
        }
    }
}

#[allow(non_snake_case)]
fn Card() -> Element {
    desktop(rsx! {
        WidgetCard { widget: BatteryWidget, timeline: Timeline::now(BatteryWidget::preview(WidgetSize::Small)), size: WidgetSize::Small, lift: LIFT() }
    })
}

#[allow(non_snake_case)]
fn Guide() -> Element {
    desktop(rsx! { WidgetSlotGuide { size: WidgetSize::Medium } })
}

#[allow(non_snake_case)]
fn Gallery() -> Element {
    let mut layout = use_signal(WidgetLayout::default);
    desktop(rsx! {
        WidgetGallery {
            layout: layout(),
            onedit: move |edit: WidgetEdit| {
                if let Ok(next) = apply(layout(), edit, GRID) {
                    layout.set(next);
                }
            },
        }
    })
}

fn harness(app: fn() -> Element) -> Harness {
    Harness::new(app, HarnessConfig::new(VIEW).with_clock(Clock::Virtual))
}

#[test]
fn a_card_lifts_and_settles_both_ways() {
    let mut harness = harness(Card);
    assert_eq!(harness.attr(".ds-widget", "data-lift"), None);
    assert_settles_to_zero_frames(&mut harness);
    harness.within(|| *LIFT.write() = Lift::Lifted);
    harness.advance(Duration::ZERO);
    assert_eq!(
        harness.attr(".ds-widget", "data-lift").as_deref(),
        Some("lifted")
    );
    assert_settles_to_zero_frames(&mut harness);
    harness.within(|| *LIFT.write() = Lift::Rest);
    harness.advance(Duration::ZERO);
    assert_eq!(harness.attr(".ds-widget", "data-lift"), None);
    assert_settles_to_zero_frames(&mut harness);
}

#[test]
fn the_slot_guide_fades_in_then_asks_for_nothing() {
    let mut harness = harness(Guide);
    assert_eq!(harness.count(".ds-widget-slot[*|data-size=medium]"), 1);
    assert_eq!(
        harness.attr(".ds-widget-slot", "aria-hidden").as_deref(),
        Some("true")
    );
    assert_settles_to_zero_frames(&mut harness);
}

#[test]
fn the_gallery_draws_one_size_and_edits_the_layout() {
    let mut harness = harness(Gallery);
    assert_eq!(harness.count(".ds-widget-gallery .ds-list-item"), 3);
    assert_eq!(
        harness.count(".ds-widget-gallery-preview .ds-widget"),
        1,
        "one preview per widget (sill Q520)"
    );
    assert_eq!(
        harness
            .attr(".ds-widget-gallery-preview .ds-widget", "data-size")
            .as_deref(),
        Some("small"),
        "the batteries at their desktop size"
    );
    assert_eq!(
        harness.count(".ds-segmented"),
        0,
        "no size control anywhere"
    );
    let clock = harness
        .centre(".ds-widget-gallery .ds-list-item:nth-child(2) .ds-row")
        .expect("World Clock in the list");
    harness.send(Input::click(clock));
    harness.advance(Duration::ZERO);
    assert_eq!(harness.count(".ds-widget-gallery-preview .ds-widget"), 1);
    assert_eq!(
        harness
            .attr(".ds-widget-gallery-preview .ds-widget", "data-widget")
            .as_deref(),
        Some("quire.world-clock")
    );
    assert_eq!(harness.count(".ds-widget-gallery-row"), 0);
    let add = harness
        .centre(".ds-widget-gallery-actions .ds-button")
        .expect("Add to Desktop");
    harness.send(Input::click(add));
    harness.advance(Duration::ZERO);
    assert_eq!(
        harness.count(".ds-widget-gallery-surface[*|data-host=desktop] .ds-widget-gallery-row"),
        1
    );
    assert_eq!(
        harness.text_of(".ds-widget-gallery-row-name").as_deref(),
        Some("World Clock")
    );
    assert_eq!(
        harness.count(".ds-widget-gallery-row .ds-segmented"),
        0,
        "a placed row has no size control"
    );
    // The new row rises in; press Remove once it has landed.
    harness.advance(Duration::from_millis(600));
    let remove = harness
        .centre(".ds-widget-gallery-row .ds-button")
        .expect("Remove");
    harness.send(Input::click(remove));
    harness.advance(Duration::ZERO);
    assert_eq!(harness.count(".ds-widget-gallery-row"), 0);
    assert_settles_to_zero_frames(&mut harness);
}
