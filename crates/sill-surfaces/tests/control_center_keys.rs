//! The control center's keyboard from the moment it opens (FINDINGS "Manual run 2026-09-28:
//! menus and keyboard"): the panel holds the keyboard, Tab walks its tiles in order, Shift+Tab
//! walks back, Space and Enter toggle the focused tile, Escape reaches the popover.

mod cc_support;

use std::time::Duration;

use cc_support::fake;
use cc_support::live::{self, Report};
use ds::Key;
use sill_services::{Radio, WifiCommand};

#[test]
fn tab_walks_the_tiles_and_space_toggles_the_focused_one() {
    let (services, feeds) = fake::services();
    let mut running = live::start(services, feeds);
    let step = Duration::from_millis(50);
    // The popover places its card a frame after it measures the anchor.
    running.harness.advance(Duration::from_millis(300));
    assert!(
        running.harness.is_focused(".sill-cc"),
        "the panel holds the keyboard as it opens"
    );
    // Wi-Fi's tile, then its chevron (the default modules: Wi-Fi, Bluetooth, Focus, ...).
    running.harness.key(Key::Tab);
    running.harness.advance(step);
    assert!(
        running.harness.is_focused(".ds-module-tile"),
        "Tab: the first tile"
    );
    running.harness.key(Key::Tab);
    running.harness.advance(step);
    assert!(
        running.harness.is_focused(".ds-module-chevron"),
        "Tab: its chevron"
    );
    running.harness.chord(&[Key::Shift], Key::Tab);
    running.harness.advance(step);
    assert!(
        running.harness.is_focused(".ds-module-tile"),
        "Shift+Tab: back on the tile"
    );
    // The fake Wi-Fi is on: Space asks for the radio off, Enter again for it off (the fake
    // service does not change), and nothing leaves the panel.
    running.harness.key(Key::Space);
    running.harness.advance(step);
    assert_eq!(
        running.feeds.wifi_commands.try_recv().ok(),
        Some(WifiCommand::Radio(Radio::Off))
    );
    running.harness.key(Key::Enter);
    running.harness.advance(step);
    assert_eq!(
        running.feeds.wifi_commands.try_recv().ok(),
        Some(WifiCommand::Radio(Radio::Off))
    );
    assert!(
        running.harness.is_focused(".ds-module-tile"),
        "the tile keeps the focus"
    );
    assert!(running.reports().is_empty(), "no page change, no close");
    running.harness.key(Key::Escape);
    running.harness.advance(Duration::from_millis(400));
    assert_eq!(running.reports().last(), Some(&Report::Closed));
}
