//! Asking before Bluetooth goes off: on the real control center in a
//! quire `Popover`, on the fake services, on the virtual clock with a pinned appearance. A press
//! that would turn Bluetooth off, on its tile or the pane's switch, shows quire's inline `Alert`
//! over the panel and sends nothing; Escape, Cancel and a press on the scrim leave it on; Turn
//! Off and Return turn it off; turning it on asks nothing. Nothing here reaches a real radio.

mod cc_support;

use cc_support::details::{center, ms, open, press};
use cc_support::live::Report;
use ds::{Point, Px};
use ds_harness::Harness;
use ds_harness::harness::{assert_settles_to_zero_frames, settle_until};
use sill_model::bluetooth::{BluetoothCommand, BtPower};

/// The Bluetooth tile: the grid's second module (Wi-Fi, Bluetooth, Focus, ...).
const TILE: &str = ".ds-module-grid > .ds-module-tile:nth-child(2)";
const ALERT: &str = ".sill-cc > .ds-alert-stage";
const CANCEL: &str = ".ds-alert-slot:last-child .ds-button";
const TURN_OFF: &str = ".ds-alert-slot:first-child .ds-button";

/// A press on the tile's disc (left of its words, well clear of its chevron).
fn press_tile(harness: &mut Harness) {
    let rect = harness.rect(TILE).expect("the Bluetooth tile");
    harness.click(Point {
        x: Px(rect.origin.x.0 + 24.0),
        y: Px(rect.origin.y.0 + rect.size.height.0 / 2.0),
    });
}

/// The alert is up, over the panel, and has sent nothing.
fn asked(running: &mut cc_support::live::Running) {
    settle_until(&mut running.harness, |h| h.count(ALERT) == 1);
    let h = &running.harness;
    assert_eq!(
        h.text_of(".ds-alert-title").as_deref(),
        Some("Turn Bluetooth off?")
    );
    assert_eq!(
        h.text_of(".ds-alert-body").as_deref(),
        Some("Bluetooth devices such as keyboards and mice will be disconnected.")
    );
    assert_eq!(h.text_of(CANCEL).as_deref(), Some("Cancel"));
    assert_eq!(h.text_of(TURN_OFF).as_deref(), Some("Turn Off"));
    assert_eq!(
        running.feeds.bluetooth_commands.try_recv().ok(),
        None,
        "asking sends nothing"
    );
    // Past the sheet's slide in (`--t-big`): a press lands where the buttons are drawn.
    running.harness.advance(ms(500));
}

/// The alert went, sending `want`.
fn answered(running: &mut cc_support::live::Running, want: Option<BluetoothCommand>) {
    settle_until(&mut running.harness, |h| h.count(ALERT) == 0);
    assert_eq!(running.feeds.bluetooth_commands.try_recv().ok(), want);
    assert_eq!(running.feeds.bluetooth_commands.try_recv().ok(), None);
}

const OFF: Option<BluetoothCommand> = Some(BluetoothCommand::Power(BtPower::Off));

#[test]
fn the_tile_asks_before_off_and_only_turn_off_or_return_sends_it() {
    let mut running = center(|_| {});
    press_tile(&mut running.harness);
    asked(&mut running);
    // It covers the panel, and the keyboard is on Turn Off, the default.
    let (stage, panel) = (
        running.harness.rect(ALERT).expect("the stage"),
        running.harness.rect(".sill-cc").expect("the panel"),
    );
    assert!(
        (stage.origin.y.0 - panel.origin.y.0).abs() < 0.5
            && (stage.size.height.0 - panel.size.height.0).abs() < 0.5
            && (stage.size.width.0 - panel.size.width.0).abs() < 0.5,
        "over the panel: {stage:?} {panel:?}"
    );
    running.harness.advance(ms(100));
    assert!(running.harness.is_focused(TURN_OFF), "Return is Turn Off");
    assert_settles_to_zero_frames(&mut running.harness);

    // Escape cancels, and the panel keeps the popover open.
    running.harness.key(ds::ShortcutKey::Escape);
    answered(&mut running, None);
    assert!(running.reports().is_empty(), "Escape stopped at the alert");

    // Cancel.
    press_tile(&mut running.harness);
    asked(&mut running);
    press(&mut running.harness, CANCEL);
    answered(&mut running, None);

    // (A press beside the alert's panel is no cancel: a sheet dims nothing and takes no click
    // for the alert; the stage only keeps the pointer from the tiles under it.)

    // Return turns it off.
    press_tile(&mut running.harness);
    asked(&mut running);
    running.harness.advance(ms(100));
    running.harness.key(ds::ShortcutKey::Enter);
    answered(&mut running, OFF);

    // Turn Off.
    press_tile(&mut running.harness);
    asked(&mut running);
    press(&mut running.harness, TURN_OFF);
    answered(&mut running, OFF);
    assert_settles_to_zero_frames(&mut running.harness);

    // The panel has the keyboard back: Escape now closes the control center.
    assert!(running.harness.is_focused(".sill-cc"));
    assert!(running.reports().is_empty());
    running.harness.key(ds::ShortcutKey::Escape);
    running.harness.advance(ms(400));
    assert_eq!(running.reports().last(), Some(&Report::Closed));
}

#[test]
fn turning_bluetooth_on_asks_nothing() {
    let mut running = center(|feeds| {
        feeds.bluetooth.send_modify(|bt| bt.power = BtPower::Off);
    });
    press_tile(&mut running.harness);
    running.harness.advance(ms(100));
    assert_eq!(running.harness.count(ALERT), 0);
    assert_eq!(
        running.feeds.bluetooth_commands.try_recv().ok(),
        Some(BluetoothCommand::Power(BtPower::On))
    );
    assert_settles_to_zero_frames(&mut running.harness);
    assert_eq!(running.harness.count(ALERT), 0);
}

#[test]
fn the_panes_switch_asks_too() {
    let mut running = center(|_| {});
    open(&mut running.harness, "Bluetooth details");
    assert_settles_to_zero_frames(&mut running.harness);
    press(&mut running.harness, ".sill-cc-detail .ds-toggle");
    asked(&mut running);
    assert_settles_to_zero_frames(&mut running.harness);
    // Escape answers the alert; the pane stays (its own Escape goes back).
    running.harness.key(ds::ShortcutKey::Escape);
    answered(&mut running, None);
    assert_eq!(
        running.harness.attr(".ds-panes", "data-shown").as_deref(),
        Some("detail")
    );
    press(&mut running.harness, ".sill-cc-detail .ds-toggle");
    asked(&mut running);
    press(&mut running.harness, TURN_OFF);
    answered(&mut running, OFF);
    assert_settles_to_zero_frames(&mut running.harness);
}
