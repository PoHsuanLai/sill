//! A desktop widget's presence on a real Blitz document and the virtual clock: the host turns
//! the card `Hidden` and keeps it drawn; the card fades out (`menu-out`, `--t-quick --e-exit`),
//! takes no pointer, and calls `on_hidden` exactly at the exit's settle, not a millisecond
//! before; the host drops it and nothing asks for a frame. Taken back before it ends, it never
//! calls `on_hidden`.

use dioxus::prelude::*;
use ds::prelude::*;
use ds_harness::harness::assert_settles_to_zero_frames;
use ds_harness::{Clock, Driver, Harness, HarnessConfig, Query, Viewport};
use sill_shell_kit::prelude::*;
use sill_shell_kit::tokens::widgets::WidgetMetrics;
use sill_shell_kit::widget::battery::BatteryWidget;
use sill_shell_kit::widget::timeline::Timeline;
use std::time::Duration;

const VIEW: Viewport = Viewport {
    width: 400,
    height: 240,
    scale_percent: 100,
};

static SHOWN: GlobalSignal<Shown> = Signal::global(|| Shown::Visible);
static GONE: GlobalSignal<u32> = Signal::global(|| 0);
static DRAWN: GlobalSignal<bool> = Signal::global(|| true);

fn desktop(motion: Motion) -> Element {
    rsx! {
        Ds { sheet: Some(sill_shell_kit::stylesheet()), appearance: Appearance { motion, ..Appearance::default() }, material: Material::Widget, chrome: Some(RootChrome::Transparent),
            div { style: "padding:20px;{WidgetMetrics::default().style_attr()}",
                if DRAWN() {
                    WidgetCard {
                        widget: BatteryWidget,
                        timeline: Timeline::now(BatteryWidget::preview(WidgetSize::Small)),
                        size: WidgetSize::Small,
                        shown: SHOWN(),
                        on_hidden: move |()| {
                            *GONE.write() += 1;
                            *DRAWN.write() = false;
                        },
                    }
                }
            }
        }
    }
}

#[allow(non_snake_case)]
fn Standard() -> Element {
    desktop(Motion::Standard)
}

#[allow(non_snake_case)]
fn Reduced() -> Element {
    desktop(Motion::Reduced)
}

fn harness(app: fn() -> Element) -> Harness {
    let mut harness = Harness::new(app, HarnessConfig::new(VIEW).with_clock(Clock::Virtual));
    harness.advance(Duration::from_millis(1200));
    harness
}

fn show(harness: &mut Harness, shown: Shown) {
    harness.within(|| *SHOWN.write() = shown);
    harness.advance(Duration::ZERO);
}

fn gone(harness: &mut Harness) -> u32 {
    harness.within(|| *GONE.peek())
}

#[test]
fn a_removed_card_fades_out_and_is_gone_at_settle() {
    let mut harness = harness(Standard);
    assert_eq!(harness.count(".ds-widget"), 1);
    assert_eq!(
        harness.attr(".ds-widget", "data-presence").as_deref(),
        Some("present")
    );
    assert_settles_to_zero_frames(&mut harness);
    show(&mut harness, Shown::Hidden);
    assert_eq!(
        harness.attr(".ds-widget", "data-presence").as_deref(),
        Some("leaving")
    );
    assert!(harness.is_animating(), "the exit is playing");
    let out = settle(Anim::MenuOut, MotionLevel::Standard);
    harness.advance(out - Duration::from_millis(1));
    assert_eq!(gone(&mut harness), 0, "not before the exit's settle");
    assert_eq!(harness.count(".ds-widget"), 1, "the host keeps it drawn");
    harness.advance(Duration::from_millis(1));
    assert_eq!(gone(&mut harness), 1, "at the exit's settle");
    assert_eq!(harness.count(".ds-widget"), 0, "the host dropped it");
    assert_settles_to_zero_frames(&mut harness);
    assert_eq!(gone(&mut harness), 1, "once");
}

#[test]
fn under_reduced_the_exit_is_still_a_short_fade() {
    let mut harness = harness(Reduced);
    show(&mut harness, Shown::Hidden);
    let out = settle(Anim::MenuOut, MotionLevel::Reduced);
    let standard = settle(Anim::MenuOut, MotionLevel::Standard);
    assert!(out <= standard, "{out:?} {standard:?}");
    harness.advance(out);
    assert_eq!(gone(&mut harness), 1);
    assert_settles_to_zero_frames(&mut harness);
}

#[test]
fn a_hide_taken_back_never_reports_hidden() {
    let mut harness = harness(Standard);
    show(&mut harness, Shown::Hidden);
    let out = settle(Anim::MenuOut, MotionLevel::Standard);
    harness.advance(out / 2);
    show(&mut harness, Shown::Visible);
    assert_eq!(
        harness.attr(".ds-widget", "data-presence").as_deref(),
        Some("present")
    );
    assert_eq!(
        harness.attr(".ds-widget", "data-pulse").as_deref(),
        Some("held"),
        "the exit's value is dropped for a hold"
    );
    harness.advance(out);
    assert_eq!(gone(&mut harness), 0);
    assert_eq!(harness.count(".ds-widget"), 1);
    assert_settles_to_zero_frames(&mut harness);
}
