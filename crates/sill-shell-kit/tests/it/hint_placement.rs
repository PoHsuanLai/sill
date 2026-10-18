//! A hint that stands beside its target (a caller-driven tooltip or dock label, a popover) is
//! never painted before it is placed: until its target's rect and its own size are measured it is
//! `visibility:hidden`, so it cannot flash at the overlay's top-left corner; once placed it stands
//! beside its target.

use dioxus::prelude::*;
use ds::prelude::*;
use ds_harness::{Clock, Harness, HarnessConfig, Viewport};
use sill_shell_kit::prelude::*;
use std::time::Duration;

const VIEW: Viewport = Viewport {
    width: 640,
    height: 400,
    scale_percent: 100,
};

#[allow(non_snake_case)]
fn Page() -> Element {
    rsx! {
        Ds { appearance: Appearance::default(), material: Material::Window, extent: RootExtent::Viewport,
            div { style: "position:absolute;left:300px;top:200px",
                Tooltip { text: "Snooze until tomorrow", shown: Some(Shown::Visible),
                    span { id: "a", style: "display:inline-block;width:60px;height:24px", "Snooze" }
                }
            }
            div { style: "position:absolute;left:100px;top:300px",
                DockLabel { text: "Files", shown: Some(Shown::Visible),
                    span { id: "b", style: "display:inline-block;width:48px;height:48px", "Files" }
                }
            }
        }
    }
}

#[test]
fn a_shown_hint_is_hidden_until_placed_and_never_at_the_origin() {
    let mut harness = Harness::new(Page, HarnessConfig::new(VIEW).with_clock(Clock::Virtual));
    for selector in [".ds-tooltip", ".ds-dock-label"] {
        let style = harness.attr(selector, "style").unwrap_or_default();
        assert!(
            style.contains("visibility:hidden"),
            "{selector} unplaced: {style}"
        );
    }
    harness.advance(Duration::from_millis(300));
    for (selector, target) in [(".ds-tooltip", "#a"), (".ds-dock-label", "#b")] {
        let style = harness.attr(selector, "style").unwrap_or_default();
        assert!(
            !style.contains("visibility:hidden"),
            "{selector} placed: {style}"
        );
        let hint = harness.rect(selector).expect("the hint");
        let target = harness.rect(target).expect("the target");
        assert!(
            (hint.origin.x.0 - target.origin.x.0).abs() < 200.0
                && (hint.origin.y.0 - target.origin.y.0).abs() < 60.0,
            "{selector} stands beside its target: {hint:?} {target:?}"
        );
    }
}

static SHIFT: GlobalSignal<u32> = Signal::global(|| 0);

#[allow(non_snake_case)]
fn Moving() -> Element {
    rsx! {
        Ds { appearance: Appearance::default(), material: Material::Window, extent: RootExtent::Viewport,
            div { style: "position:absolute;left:300px;top:{200 + SHIFT()}px",
                Tooltip { text: "Snooze until tomorrow", shown: Some(Shown::Visible),
                    span { id: "a", style: "display:inline-block;width:60px;height:24px", "Snooze" }
                }
            }
        }
    }
}

/// A hint follows its target's latest rect: when layout moves the target after the hint was
/// placed, the hint is placed again beside it.
#[test]
fn a_placed_hint_follows_its_target_when_the_target_moves() {
    let mut harness = Harness::new(Moving, HarnessConfig::new(VIEW).with_clock(Clock::Virtual));
    harness.advance(Duration::from_millis(400));
    let before = harness.rect(".ds-tooltip").expect("the tooltip").origin.y.0;
    harness.within(|| *SHIFT.write() = 60);
    harness.advance(Duration::from_millis(600));
    let target = harness.rect("#a").expect("the target");
    let hint = harness.rect(".ds-tooltip").expect("the tooltip");
    assert!(
        hint.origin.y.0 > before + 40.0,
        "it moved with the target: {before} -> {hint:?}"
    );
    assert!(
        (hint.origin.y.0 - (target.origin.y.0 + target.size.height.0)).abs() < 12.0,
        "and stands under it: {hint:?} {target:?}"
    );
}

/// Frame by frame on the virtual clock, a hint is either hidden or beside its target: never at the
/// origin, never over the target.
#[test]
fn a_hint_is_never_painted_at_the_origin_or_over_its_target_on_any_frame() {
    let mut harness = Harness::new(Moving, HarnessConfig::new(VIEW).with_clock(Clock::Virtual));
    let mut shown_frames = 0;
    for _ in 0..60 {
        harness.advance(Duration::from_millis(16));
        let Some(style) = harness.attr(".ds-tooltip", "style") else {
            continue;
        };
        if style.contains("visibility:hidden") {
            continue;
        }
        shown_frames += 1;
        let hint = harness.rect(".ds-tooltip").expect("the tooltip");
        let target = harness.rect("#a").expect("the target");
        assert!(
            hint.origin.x.0 > 1.0 || hint.origin.y.0 > 1.0,
            "at the origin: {hint:?}"
        );
        assert!(
            hint.origin.y.0 >= target.origin.y.0 + target.size.height.0 - 1.0
                || hint.origin.y.0 + hint.size.height.0 <= target.origin.y.0 + 1.0,
            "over its target: {hint:?} {target:?}"
        );
    }
    assert!(shown_frames > 0, "it was placed and shown");
}
