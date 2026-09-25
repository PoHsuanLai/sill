//! The hot corner surface in a `ds_native::Harness` (design/13 §13.3.12, FINDINGS "M10 freeze"
//! F409): a real pointer dwelling in it fires once, not before the dwell and not twice without a
//! leave and the re-arm; and its stylesheet is lint-clean under `Profile::Strict` (coherence
//! rule 1, quire CONSUMING.md §5).

mod support;

use std::cell::RefCell;
use std::time::Duration;

use dioxus::prelude::*;
use ds::lint::{LintConfig, Profile, assert_clean};
use ds_native::{Harness, Viewport};
use ds_settings::Ms;
use shell_host::OutputId;
use sill_settings::{CornerAction, CornerModifier, HotCornersSettings, SettingsFeed, ShellFile};
use sill_surfaces::HotCorner;
use sill_surfaces::surfaces::hot_corners::{Corner, CornerLink, HOT_CORNERS_CSS};

const DWELL_MS: u16 = 40;
const REARM_MS: u16 = 60;

fn settings(modifier: CornerModifier) -> HotCornersSettings {
    HotCornersSettings {
        top_left: CornerAction::Launcher,
        dwell_ms: Ms(DWELL_MS),
        rearm_ms: Ms(REARM_MS),
        modifier,
        ..HotCornersSettings::default()
    }
}

thread_local! {
    static WORLD: RefCell<Option<(CornerLink, ShellFile)>> = const { RefCell::new(None) };
}

fn app() -> Element {
    let Some((link, file)) = WORLD.with(|w| w.borrow().clone()) else {
        return rsx! {};
    };
    use_context_provider(|| link);
    use_context_provider(|| SettingsFeed::fixed(file));
    rsx! {
        HotCorner {
            output: support::fake::output(),
            corner: Corner::TopLeft,
        }
    }
}

/// The corner mounted on `hot_corners`, in a small viewport (the real surface is
/// `hot_corners.size_px` square; the test's viewport only needs to be big enough that a point
/// well outside the corner's own `100vw x 100vh` element is still a point the harness can move
/// the pointer to for a leave).
fn start(hot_corners: HotCornersSettings) -> (Harness, CornerLink) {
    let link = CornerLink::new();
    let file = ShellFile {
        hot_corners,
        ..ShellFile::default()
    };
    WORLD.with(|w| *w.borrow_mut() = Some((link.clone(), file)));
    let viewport = Viewport {
        width: 4,
        height: 4,
        scale_percent: 100,
    };
    (Harness::new(app, viewport), link)
}

/// The corner element's own centre (the UA's default margin around `main` means it is not at
/// the viewport's origin).
fn inside(harness: &Harness) -> ds::Point {
    harness
        .centre(".sill-hot-corner")
        .expect("the corner element is laid out")
}

/// Well clear of the corner element, still inside the viewport.
fn outside(harness: &Harness) -> ds::Point {
    let at = inside(harness);
    ds::Point {
        x: at.x + ds::Px(200.0),
        y: at.y + ds::Px(200.0),
    }
}

#[test]
fn a_dwell_fires_once_and_not_before() {
    let (mut harness, link) = start(settings(CornerModifier::None));
    harness.advance(Duration::from_millis(10));
    assert_eq!(link.last(), None);
    harness.pointer_move(inside(&harness));
    harness.advance(Duration::from_millis(DWELL_MS as u64 / 2));
    assert_eq!(link.last(), None, "not before the dwell");
    harness.advance(Duration::from_millis(DWELL_MS as u64));
    let first = link.last().expect("fired after the dwell");
    assert_eq!(first.corner, Corner::TopLeft);
    assert_eq!(first.output, OutputId(support::fake::output().id.0));
    // Staying inside fires nothing more.
    harness.advance(Duration::from_millis(DWELL_MS as u64 * 3));
    assert_eq!(
        link.last(),
        Some(first),
        "a pointer parked in the corner acts once"
    );
}

#[test]
fn leaving_before_the_dwell_cancels_it() {
    let (mut harness, link) = start(settings(CornerModifier::None));
    harness.pointer_move(inside(&harness));
    harness.advance(Duration::from_millis(DWELL_MS as u64 / 2));
    harness.pointer_move(outside(&harness));
    harness.advance(Duration::from_millis(DWELL_MS as u64 * 3));
    assert_eq!(link.last(), None, "left before the dwell ran out");
}

#[test]
fn leaving_and_the_rearm_lets_it_fire_again() {
    let (mut harness, link) = start(settings(CornerModifier::None));
    harness.pointer_move(inside(&harness));
    harness.advance(Duration::from_millis(DWELL_MS as u64 * 2));
    let first = link.last().expect("the first dwell fired");
    assert_eq!(first.seq, 1);
    // Leaving well before the re-arm is over and coming back starts no new dwell at all (only a
    // leave while re-armed does; entering while `Spent` just notes the pointer is back in), so
    // however long it then stays inside, nothing fires again.
    harness.pointer_move(outside(&harness));
    harness.advance(Duration::from_millis(REARM_MS as u64 / 4));
    harness.pointer_move(inside(&harness));
    harness.advance(Duration::from_millis(DWELL_MS as u64 * 5));
    assert_eq!(
        link.last(),
        Some(first),
        "still within the re-arm, no fresh dwell was armed"
    );
    // Leaving again for past the re-arm, then a fresh dwell fires once more.
    harness.pointer_move(outside(&harness));
    harness.advance(Duration::from_millis(REARM_MS as u64 * 3));
    harness.pointer_move(inside(&harness));
    harness.advance(Duration::from_millis(DWELL_MS as u64 * 3));
    let second = link.last().expect("re-armed");
    assert_eq!(
        second.seq, 2,
        "a fresh firing after the leave and the re-arm"
    );
}

#[test]
fn a_required_modifier_blocks_the_dwell_until_it_is_held() {
    let (mut harness, link) = start(settings(CornerModifier::Shift));
    harness.pointer_move(inside(&harness));
    harness.advance(Duration::from_millis(DWELL_MS as u64 * 3));
    assert_eq!(link.last(), None, "shift was never reported held");
    harness.pointer_move(outside(&harness));
    harness.advance(Duration::from_millis(REARM_MS as u64 * 2));
    harness.pointer_move_with(inside(&harness), dioxus::prelude::Modifiers::SHIFT);
    harness.advance(Duration::from_millis(DWELL_MS as u64 * 3));
    assert!(
        link.last().is_some(),
        "fires once the harness reports shift held"
    );
}

#[test]
fn the_stylesheet_is_clean_under_strict() {
    let strict = LintConfig {
        profile: Profile::Strict,
        ..LintConfig::default()
    };
    assert_clean(HOT_CORNERS_CSS, &strict);
}
