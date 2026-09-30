//! The Display module's brightness capsule on the real control center: a pointer drag and a
//! click on the capsule send the displays
//! service the level under the pointer for the bar's output, and a level the service publishes
//! from elsewhere (a key, the monitor's own buttons) moves the capsule. Fake services on
//! `Clock::Virtual`, the appearance pinned; every case ends at 0 frames. Nothing here reaches a
//! monitor.

mod cc_support;

use cc_support::details::{center, center_at, ms};

use ds::{Point, Px};
use ds_harness::Harness;
use ds_harness::harness::assert_settles_to_zero_frames;
use sill_model::displays::ddc::Endpoint;
use sill_model::displays::{
    Controls, DdcDisplay, Displays, DisplaysCommand, Following, Level, LevelChange,
};
use sill_model::ids::{DdcId, OutputName};
use sill_model::units::Percent;
use tokio::sync::mpsc::UnboundedReceiver;

const CAPSULE: &str = ".ds-slider[*|aria-label=\"Display\"]";
/// A module panel dimmed with its level (Display's, the one disabled here).
const DISABLED_PANEL: &str = ".ds-module-panel[*|aria-disabled=true]";
const RAIL: &str = ".ds-slider[*|aria-label=\"Display\"] .ds-slider-rail";

/// A monitor on `DP-1` (the panel's bar output) answering DDC at `level` of 100.
fn monitor(level: u16) -> Displays {
    Displays {
        ddc: vec![DdcDisplay {
            endpoint: Endpoint {
                id: DdcId(7),
                connector: Some(OutputName("DP-1".to_owned())),
                product: "Fake Monitor".to_owned(),
            },
            controls: Controls {
                brightness: Some(Level {
                    value: level,
                    max: 100,
                }),
                ..Controls::default()
            },
        }],
        ..Displays::default()
    }
}

/// Every brightness level the panel asked for so far, in order.
fn levels(commands: &mut UnboundedReceiver<DisplaysCommand>) -> Vec<u16> {
    std::iter::from_fn(|| commands.try_recv().ok())
        .filter_map(|command| match command {
            DisplaysCommand::SetBrightness { output, level } => {
                assert_eq!(output, OutputName("DP-1".to_owned()), "the bar's output");
                Some(level.0)
            }
            DisplaysCommand::Brightness {
                change: LevelChange::Set(Percent(level)),
                ..
            } => Some(level),
            _ => None,
        })
        .collect()
}

/// The point `fraction` of the way along the knob's travel, on the rail's middle line: the
/// knob's centre travels from half the capsule's height in at each end (design/29, the
/// control center's `CapsuleKnob`).
fn along(harness: &Harness, fraction: f32) -> Point {
    let rail = harness.rect(RAIL).expect("the Display capsule is laid out");
    let inset = rail.size.height.0 / 2.0;
    Point {
        x: Px(rail.origin.x.0 + inset + (rail.size.width.0 - 2.0 * inset) * fraction),
        y: Px(rail.origin.y.0 + rail.size.height.0 / 2.0),
    }
}

fn shown(harness: &Harness) -> Option<String> {
    harness.attr(CAPSULE, "aria-valuenow")
}

#[test]
fn a_drag_on_the_capsule_follows_the_pointer_to_the_service() {
    drag_at(100);
}

/// The user's BenQ MA270U runs at scale 2.
#[test]
fn a_drag_follows_the_pointer_at_scale_2() {
    drag_at(200);
}

fn drag_at(scale_percent: u16) {
    let mut running = center_at(scale_percent, |feeds| {
        feeds.displays.send_replace(monitor(40));
    });
    let harness = &mut running.harness;
    assert_eq!(shown(harness).as_deref(), Some("40"));
    assert_eq!(harness.attr(CAPSULE, "aria-disabled"), None, "enabled");
    let from = along(harness, 0.40);
    let to = along(harness, 0.80);
    harness.pointer_move(from);
    harness.pointer_down(from);
    // The rail is measured after the press, in a task.
    harness.advance(ms(16));
    for step in 1..=8u8 {
        let part = f32::from(step) / 8.0;
        harness.pointer_move(Point {
            x: Px(from.x.0 + (to.x.0 - from.x.0) * part),
            y: from.y,
        });
        harness.advance(ms(16));
    }
    harness.pointer_up(to);
    harness.advance(ms(50));
    let sent = levels(&mut running.feeds.displays_commands);
    assert!(
        sent.len() >= 4,
        "the level follows the drag, not only its end: {sent:?}"
    );
    assert!(
        sent.windows(2).all(|pair| pair[0] < pair[1]),
        "rising with the pointer: {sent:?}"
    );
    let last = *sent.last().expect("a level");
    assert!(
        (78..=82).contains(&last),
        "ends under the pointer: {sent:?}"
    );
    assert_settles_to_zero_frames(&mut running.harness);
}

#[test]
fn a_click_on_the_track_jumps_there() {
    click_at(100);
}

#[test]
fn a_click_jumps_there_at_scale_2() {
    click_at(200);
}

fn click_at(scale_percent: u16) {
    let mut running = center_at(scale_percent, |feeds| {
        feeds.displays.send_replace(monitor(40));
    });
    let at = along(&running.harness, 0.25);
    running.harness.click(at);
    running.harness.advance(ms(50));
    let sent = levels(&mut running.feeds.displays_commands);
    assert_eq!(sent.len(), 1, "one jump: {sent:?}");
    assert!(
        (23..=27).contains(&sent[0]),
        "where the click was: {sent:?}"
    );
    assert_settles_to_zero_frames(&mut running.harness);
}

#[test]
fn a_level_set_elsewhere_moves_the_capsule() {
    let mut running = center(|feeds| {
        feeds.displays.send_replace(monitor(40));
    });
    running.feeds.displays.send_replace(monitor(70));
    running.harness.advance(ms(400));
    assert_eq!(shown(&running.harness).as_deref(), Some("70"));
    assert_settles_to_zero_frames(&mut running.harness);
}

#[test]
fn the_open_module_asks_the_service_to_follow_the_monitor() {
    let mut running = center(|feeds| {
        feeds.displays.send_replace(monitor(40));
    });
    running.harness.advance(ms(50));
    let asked: Vec<DisplaysCommand> =
        std::iter::from_fn(|| running.feeds.displays_commands.try_recv().ok()).collect();
    assert_eq!(asked, vec![DisplaysCommand::Follow(Following::Start)]);
    assert_settles_to_zero_frames(&mut running.harness);
}

#[test]
fn a_monitor_that_answers_late_brings_the_capsule_alive() {
    // The startup probe found no monitor: the capsule is disabled at 0, and a press does nothing.
    let mut running = center(|_| {});
    assert_eq!(shown(&running.harness).as_deref(), Some("0"));
    assert_eq!(
        running.harness.attr(CAPSULE, "aria-disabled").as_deref(),
        Some("true")
    );
    // Its panel dims with it (the header too), and the capsule is out of the tab order.
    assert_eq!(running.harness.count(DISABLED_PANEL), 1, "the panel dims");
    assert_eq!(
        running.harness.attr(CAPSULE, "tabindex"),
        None,
        "no tab stop"
    );
    let at = along(&running.harness, 0.5);
    running.harness.click(at);
    running.harness.advance(ms(50));
    assert_eq!(
        levels(&mut running.feeds.displays_commands),
        Vec::<u16>::new()
    );
    // The service's probe while the module follows finds it: the capsule takes input.
    running.feeds.displays.send_replace(monitor(40));
    running.harness.advance(ms(400));
    assert_eq!(shown(&running.harness).as_deref(), Some("40"));
    assert_eq!(running.harness.attr(CAPSULE, "aria-disabled"), None);
    assert_eq!(running.harness.count(DISABLED_PANEL), 0, "the panel lit");
    running.harness.click(at);
    running.harness.advance(ms(50));
    let sent = levels(&mut running.feeds.displays_commands);
    assert!(
        sent.len() == 1 && (48..=52).contains(&sent[0]),
        "the click reaches the monitor: {sent:?}"
    );
    assert_settles_to_zero_frames(&mut running.harness);
}
