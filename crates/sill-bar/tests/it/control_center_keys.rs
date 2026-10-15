//! The control center's keyboard from the moment it opens: the panel holds the keyboard, Tab
//! walks its tiles in order, Shift+Tab walks back, Space and Enter toggle the focused tile,
//! Escape reaches the popover, also after a press on nothing that takes the keyboard.

use crate::cc_support;

use ds_harness::{Driver, FocusState, Input, Query};
use std::time::Duration;

use cc_support::fake;
use cc_support::live::{self, Report};
use ds::prelude::*;
use sill_model::wifi::{Radio, WifiCommand};

#[test]
fn tab_walks_the_tiles_and_space_toggles_the_focused_one() {
    let (services, feeds) = fake::services();
    let mut running = live::start(services, feeds);
    let step = Duration::from_millis(50);
    // The popover places its card a frame after it measures the anchor.
    running.harness.advance(Duration::from_millis(300));
    assert_eq!(
        running.harness.focus_of(".sill-cc"),
        FocusState::Focused,
        "the panel holds the keyboard as it opens"
    );
    // Wi-Fi's tile, then its chevron (the default modules: Wi-Fi, Bluetooth, Focus, ...).
    running.harness.send(Input::key(ShortcutKey::Tab));
    running.harness.advance(step);
    assert_eq!(
        running.harness.focus_of(".ds-module-tile"),
        FocusState::Focused,
        "Tab: the first tile"
    );
    running.harness.send(Input::key(ShortcutKey::Tab));
    running.harness.advance(step);
    assert_eq!(
        running.harness.focus_of(".ds-module-chevron"),
        FocusState::Focused,
        "Tab: its chevron"
    );
    running
        .harness
        .send(Input::chord(&[ShortcutKey::Shift], ShortcutKey::Tab));
    running.harness.advance(step);
    assert_eq!(
        running.harness.focus_of(".ds-module-tile"),
        FocusState::Focused,
        "Shift+Tab: back on the tile"
    );
    // The fake Wi-Fi is on: Space asks for the radio off, Enter again for it off (the fake
    // service does not change), and nothing leaves the panel.
    running.harness.send(Input::key(ShortcutKey::Space));
    running.harness.advance(step);
    assert_eq!(
        running.feeds.wifi_commands.try_recv().ok(),
        Some(WifiCommand::Radio(Radio::Off))
    );
    running.harness.send(Input::key(ShortcutKey::Enter));
    running.harness.advance(step);
    assert_eq!(
        running.feeds.wifi_commands.try_recv().ok(),
        Some(WifiCommand::Radio(Radio::Off))
    );
    assert_eq!(
        running.harness.focus_of(".ds-module-tile"),
        FocusState::Focused,
        "the tile keeps the focus"
    );
    assert!(running.reports().is_empty(), "no page change, no close");
    running.harness.send(Input::key(ShortcutKey::Escape));
    running.harness.advance(Duration::from_millis(400));
    assert_eq!(running.reports().last(), Some(&Report::Closed));
}

#[test]
fn escape_closes_after_a_press_on_nothing_that_takes_the_keyboard() {
    let (services, feeds) = fake::services();
    let mut running = live::start_in_shell(services, feeds);
    running.harness.advance(Duration::from_millis(300));
    assert_eq!(
        running.harness.focus_of(".sill-cc"),
        FocusState::Focused,
        "the panel holds the keyboard as it opens"
    );
    // The panel's own padding: nothing there takes a press, so Blitz clears the focus.
    let panel = running
        .harness
        .rect(".sill-cc")
        .expect("the panel is laid out");
    let padding = Point {
        x: Px(panel.origin.x.0 + 2.0),
        y: Px(panel.origin.y.0 + 2.0),
    };
    running.harness.send(Input::click(padding));
    running.harness.advance(Duration::from_millis(100));
    assert_eq!(
        running.harness.focus_of(".sill-cc"),
        FocusState::Focused,
        "the panel took the keyboard back"
    );
    running.harness.send(Input::key(ShortcutKey::Escape));
    running.harness.advance(Duration::from_millis(400));
    assert_eq!(running.reports().last(), Some(&Report::Closed));
}
