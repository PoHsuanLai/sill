//! The bar gaps' menu controls on a real Blitz document (FINDINGS "Bar gaps"): the
//! menu reports the choice under the pointer and every release over it; a press that began
//! outside and is released on an item picks it; a pick calls `onpick` before `onclose`; Escape
//! and an outside click fade the menu out before `onclose`; every menu opens with no entrance;
//! a status line is drawn and never selected; a press reports where it happened.

use dioxus::prelude::*;
use ds::base::press::{PointerButton, Press};
use ds::components::controls::button_model::ImagePosition;
use ds::host::measure::Anchor;
use ds::host::measure::use_rect;
use ds::prelude::*;
use ds_harness::harness::settle_until;
use ds_harness::{Clock, Driver, Harness, HarnessConfig, Input, Query, Viewport};
use sill_shell_kit::prelude::*;
use std::time::Duration;

const VIEW: Viewport = Viewport {
    width: 480,
    height: 360,
    scale_percent: 100,
};

fn ms(n: u64) -> Duration {
    Duration::from_millis(n)
}

fn item(value: u8, title: &str, availability: Availability) -> MenuItem<u8> {
    MenuItem::new(value, title).with_availability(availability)
}

/// A status line, then three choices: Open (0), Pause (1, disabled), Quit (2).
fn entries() -> Vec<MenuItem<u8>> {
    vec![
        MenuItem::Info {
            title: "Wired".to_string(),
            detail: Some("192.168.1.4".to_string()),
        },
        item(1, "Open", Availability::Enabled),
        item(2, "Pause", Availability::Disabled),
        item(3, "Quit", Availability::Enabled),
    ]
}

/// Whether the menu under test starts open.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Setup {
    open: bool,
}

/// A page with an opener that opens the menu on the press (as a bar title does), the menu, and
/// a log of what the menu reported, in order.
#[component]
fn MenuPage(setup: Setup) -> Element {
    let mut open = use_signal(|| setup.open);
    let mut log = use_signal(Vec::<String>::new);
    let mut note = move |line: String| log.with_mut(|log| log.push(line));
    rsx! {
        Ds { appearance: Appearance::default(), material: Material::Window,
            div { style: "display:flex; flex-direction:column; height:340px",
                div {
                    class: "opener",
                    style: "width:80px; height:24px",
                    onmousedown: move |_| open.set(true),
                }
                p { class: "log", {log().join(",")} }
            }
            if open() {
                Menu::<u8> {
                    placement: MenuPlacement::Bar,
                    anchor: Anchor::Point(Point { x: Px(40.0), y: Px(60.0) }),
                    items: entries(),
                    on_hover: move |index: Option<usize>| {
                        note(format!("hover:{}", index.map_or("none".to_string(), |i| i.to_string())));
                    },
                    on_release: move |press: Press| note(format!("release:{:?}", press.button)),
                    onpick: move |value: u8| note(format!("pick:{value}")),
                    onclose: move |()| {
                        note("close".to_string());
                        open.set(false);
                    },
                }
            }
        }
    }
}

#[allow(non_snake_case)]
fn OpenMenu() -> Element {
    rsx! { MenuPage { setup: Setup { open: true } } }
}

#[allow(non_snake_case)]
fn ClosedMenu() -> Element {
    rsx! { MenuPage { setup: Setup { open: false } } }
}

fn log(harness: &Harness) -> String {
    harness.text_of(".log").unwrap_or_default()
}

/// The centre of the row titled `title`.
fn row(harness: &Harness, title: &str) -> Point {
    let index = ["Open", "Pause", "Quit"]
        .iter()
        .position(|t| *t == title)
        .unwrap_or_else(|| panic!("no row {title}"));
    harness
        // The status line is the menu's first child.
        .centre(&format!(".ds-menu > :nth-child({})", index + 2))
        .unwrap_or_else(|| panic!("{title} is not drawn:\n{}", harness.html()))
}

/// Let the menu mount, measure and place itself.
fn settle_in(harness: &mut Harness) {
    harness.advance(ms(80));
}

#[test]
fn the_menu_reports_the_choice_under_the_pointer() {
    let mut harness = Harness::new(
        OpenMenu,
        HarnessConfig::new(VIEW).with_clock(Clock::Virtual),
    );
    settle_in(&mut harness);
    harness.send(Input::pointer_move(row(&harness, "Open")));
    harness.send(Input::pointer_move(row(&harness, "Quit")));
    let info = harness.centre(".ds-menu-info").expect("the status line");
    harness.send(Input::pointer_move(info));
    assert_eq!(log(&harness), "hover:0,hover:2,hover:none");
}

#[test]
fn a_status_line_is_drawn_and_never_selected() {
    let mut harness = Harness::new(
        OpenMenu,
        HarnessConfig::new(VIEW).with_clock(Clock::Virtual),
    );
    settle_in(&mut harness);
    assert_eq!(
        harness.text_of(".ds-menu-info-title").as_deref(),
        Some("Wired")
    );
    assert_eq!(
        harness.text_of(".ds-menu-info-detail").as_deref(),
        Some("192.168.1.4")
    );
    assert_eq!(
        harness.count(".ds-menu-item"),
        3,
        "a status line is no item"
    );
    // Up from the first choice wraps to the last, past the status line.
    harness.send(Input::key(ShortcutKey::Up));
    assert_eq!(
        harness
            .text_of(".ds-menu-item[*|data-selected=true] .ds-menu-label")
            .as_deref(),
        Some("Quit")
    );
    harness.send(Input::key(ShortcutKey::Down));
    assert_eq!(
        harness
            .text_of(".ds-menu-item[*|data-selected=true] .ds-menu-label")
            .as_deref(),
        Some("Open")
    );
}

/// design/13 section 13.3.2: while the button is held, release on an enabled item picks it.
#[test]
fn a_press_dragged_onto_an_item_and_released_picks_it() {
    let mut harness = Harness::new(
        ClosedMenu,
        HarnessConfig::new(VIEW).with_clock(Clock::Virtual),
    );
    let opener = harness.centre(".opener").expect("the opener");
    harness.send(Input::pointer_move(opener));
    harness.send(Input::pointer_down(opener));
    settle_in(&mut harness);
    assert_eq!(harness.count(".ds-menu"), 1, "the press opened the menu");
    let quit = row(&harness, "Quit");
    harness.send(Input::pointer_move(quit));
    harness.send(Input::pointer_up(quit));
    settle_until(&mut harness, |h| log(h).ends_with("close"));
    assert_eq!(log(&harness), "hover:2,release:Primary,pick:3,close");
    assert_eq!(harness.count(".ds-menu"), 0, "the pick closed it");
}

/// A drag released on a disabled item picks nothing and closes, through the exit fade.
#[test]
fn a_drag_released_on_a_disabled_item_closes_picking_nothing() {
    let mut harness = Harness::new(
        ClosedMenu,
        HarnessConfig::new(VIEW).with_clock(Clock::Virtual),
    );
    let opener = harness.centre(".opener").expect("the opener");
    harness.send(Input::pointer_move(opener));
    harness.send(Input::pointer_down(opener));
    settle_in(&mut harness);
    let pause = row(&harness, "Pause");
    harness.send(Input::pointer_move(pause));
    harness.send(Input::pointer_up(pause));
    harness.advance(fade() + ms(40));
    assert_eq!(log(&harness), "hover:1,release:Primary,close");
}

/// A click that starts and ends on an item is a click: it picks, and a release after a press
/// inside the menu picks nothing by itself. `onpick` runs before `onclose`.
#[test]
fn a_click_picks_before_it_closes() {
    let mut harness = Harness::new(
        OpenMenu,
        HarnessConfig::new(VIEW).with_clock(Clock::Virtual),
    );
    settle_in(&mut harness);
    harness.send(Input::click(row(&harness, "Open")));
    settle_until(&mut harness, |h| log(h).ends_with("close"));
    assert_eq!(log(&harness), "hover:0,release:Primary,pick:1,close");
}

/// How long the exit fade runs, asked of the motion table.
fn fade() -> Duration {
    settle(Anim::MenuOut, MotionLevel::Standard)
}

/// Escape fades the menu out (`data-presence="leaving"`) and calls `onclose` when the fade has
/// settled, not before.
#[test]
fn escape_fades_the_menu_out_before_it_closes() {
    let mut harness = Harness::new(
        OpenMenu,
        HarnessConfig::new(VIEW).with_clock(Clock::Virtual),
    );
    settle_in(&mut harness);
    let escaped = harness.now();
    harness.send(Input::key(ShortcutKey::Escape));
    assert_eq!(
        harness.attr(".ds-menu", "data-presence").as_deref(),
        Some("leaving")
    );
    // Half the fade, not `fade() - 40ms` (fixed 2026-09-25, FINDINGS "Timing tests"): the old
    // margin was only 40 ms of a 170 ms window, so a loaded machine's overshoot on `advance`
    // (it guarantees *at least* the time asked for, never exactly it) could cross the boundary
    // before this read.
    harness.advance(fade() / 2);
    assert_eq!(log(&harness), "", "still fading at half the fade");
    assert_eq!(harness.count(".ds-menu"), 1);
    let closed = settle_until(&mut harness, |h| log(h) == "close");
    assert_eq!(
        closed.duration_since(escaped),
        fade(),
        "onclose landed only once the full fade had run"
    );
    assert_eq!(harness.count(".ds-menu"), 0);
}

/// An outside click fades the menu out too.
#[test]
fn an_outside_click_fades_the_menu_out_before_it_closes() {
    let mut harness = Harness::new(
        OpenMenu,
        HarnessConfig::new(VIEW).with_clock(Clock::Virtual),
    );
    settle_in(&mut harness);
    harness.send(Input::click(Point {
        x: Px(440.0),
        y: Px(320.0),
    }));
    assert_eq!(
        harness.attr(".ds-menu", "data-presence").as_deref(),
        Some("leaving")
    );
    assert_eq!(log(&harness), "");
    harness.advance(fade() + ms(40));
    assert_eq!(log(&harness), "close");
}

/// Every menu opens at rest, with no entrance.
#[test]
fn a_menu_opens_with_no_entrance() {
    let harness = Harness::new(
        OpenMenu,
        HarnessConfig::new(VIEW).with_clock(Clock::Virtual),
    );
    assert_eq!(
        harness.attr(".ds-menu", "data-presence").as_deref(),
        Some("present")
    );
}

// ---- Where a press happened ---------------------------------------------------------------

#[allow(non_snake_case)]
fn PressAt() -> Element {
    let mut seen = use_signal(|| "none".to_string());
    rsx! {
        Ds { sheet: Some(sill_shell_kit::stylesheet()), appearance: Appearance::default(), material: Material::Bar,
            div { style: "display:flex; padding:20px 0 0 100px; height:60px",
                MenuBarItem {
                    image: ImagePosition::Only,
                    icon: Icon::Wifi,
                    label: "Network",
                    onclick: move |press: Press| {
                        seen.set(format!("{:?} {} {}", press.button, press.at.x.0, press.at.y.0));
                    },
                }
            }
            p { class: "seen", "{seen}" }
        }
    }
}

/// A press carries its surface-local point, for SNI's `Activate(x, y)` and `ContextMenu(x, y)`.
#[test]
fn a_press_reports_where_it_happened() {
    let mut harness = Harness::new(PressAt, HarnessConfig::new(VIEW).with_clock(Clock::Virtual));
    // (point, button)
    const CASES: &[(f32, f32, PointerButton)] = &[
        (105.0, 25.0, PointerButton::Primary),
        (115.0, 35.0, PointerButton::Secondary),
    ];
    for &(x, y, button) in CASES {
        harness.send(Input::press(Point { x: Px(x), y: Px(y) }, button));
        assert_eq!(
            harness.text_of(".seen").as_deref(),
            Some(format!("{button:?} {x} {y}").as_str())
        );
    }
}

// ---- The exported measurer ----------------------------------------------------------------

#[allow(non_snake_case)]
fn MeasuredRoot() -> Element {
    // What a shell-host surface's root does, where `ds_blitz::launch` did not provide it.
    ds_blitz::provide_host();
    let probe = use_rect();
    let width = probe.rect().map_or(0.0, |rect| rect.size.width.0);
    rsx! {
        Ds { appearance: Appearance::default(), material: Material::Bar,
            div { style: "display:flex",
                div { style: "width:123px; height:10px", onmounted: move |event| probe.on_mounted(event) }
            }
            p { class: "width", "{width}" }
        }
    }
}

/// `ds_blitz::provide_host()` gives a root the Blitz rect read `use_rect` goes through.
#[test]
fn the_exported_measurer_reads_a_rect() {
    let mut harness = Harness::new(
        MeasuredRoot,
        HarnessConfig::new(VIEW).with_clock(Clock::Virtual),
    );
    harness.advance(ms(80));
    assert_eq!(harness.text_of(".width").as_deref(), Some("123"));
}

/// The title of the selected choice.
fn selected(harness: &Harness) -> Option<String> {
    harness.text_of(".ds-menu-item[*|data-selected=true] .ds-menu-label")
}

#[test]
fn home_and_end_go_to_the_first_and_last_enabled_choice() {
    let mut harness = Harness::new(
        OpenMenu,
        HarnessConfig::new(VIEW).with_clock(Clock::Virtual),
    );
    settle_in(&mut harness);
    harness.send(Input::key(ShortcutKey::End));
    assert_eq!(selected(&harness).as_deref(), Some("Quit"));
    harness.send(Input::key(ShortcutKey::Home));
    assert_eq!(selected(&harness).as_deref(), Some("Open"));
}

#[test]
fn typing_a_letter_selects_the_choice_that_starts_with_it() {
    let mut harness = Harness::new(
        OpenMenu,
        HarnessConfig::new(VIEW).with_clock(Clock::Virtual),
    );
    settle_in(&mut harness);
    harness.send(Input::key(ShortcutKey::Char('q')));
    assert_eq!(selected(&harness).as_deref(), Some("Quit"));
    harness.advance(ms(1200));
    harness.send(Input::key(ShortcutKey::Char('o')));
    assert_eq!(
        selected(&harness).as_deref(),
        Some("Open"),
        "the buffer was forgotten"
    );
    harness.advance(ms(1200));
    harness.send(Input::key(ShortcutKey::Char('p')));
    assert_eq!(
        selected(&harness).as_deref(),
        Some("Open"),
        "a disabled choice is not typed onto"
    );
}
