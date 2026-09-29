//! The low-battery nudge (quire design/26 5.1.3) on the real bar in a Blitz document: the
//! battery item nudges once
//! when a discharging battery crosses `bar.battery_low_percent`, never for a wobble back over
//! it, again only after it was plugged in, and the bar paints 0 frames after each change.

use std::time::Duration;

use ds_native::harness::{assert_settles_to_zero_frames, settle_until};
use ds_native::{Harness, Viewport};
use sill_model::battery::{Battery, Charge};
use sill_model::units::{Percent, Presence};
use sill_ui_kit::Bar;
use sill_ui_kit::harness::SurfaceHarness;
use sill_ui_kit::harness::fake;
use sill_ui_kit::links::ControlCenterLink;

const VIEW: Viewport = Viewport {
    width: 1280,
    height: 40,
    scale_percent: 100,
};

/// quire's nudge inside the item's button: it lifts the glyph, never the pill.
const NUDGE: &str = ".ds-icon-button > .ds-icon-nudge.a-nudge-up";

fn at(level: u16, charge: Charge) -> Battery {
    Battery {
        presence: Presence::Present,
        level: Percent(level),
        charge,
        ..Battery::default()
    }
}

/// Feeds `battery` and says whether the item nudged within the next second.
fn feed(harness: &mut Harness, feeds: &fake::Feeds, battery: Battery) -> bool {
    feeds.battery.send_replace(battery);
    let mut seen = false;
    for _ in 0..20 {
        harness.advance(Duration::from_millis(50));
        seen |= harness.count(NUDGE) > 0;
        assert!(
            !harness.has_class(".ds-icon-button", "a-nudge-up"),
            "the pill stays"
        );
    }
    assert_settles_to_zero_frames(harness);
    seen
}

#[test]
fn the_battery_item_nudges_once_per_crossing() {
    let SurfaceHarness {
        mut harness, feeds, ..
    } = SurfaceHarness::<Bar>::setup(ControlCenterLink::new(), ())
        .viewport(VIEW)
        .start();
    settle_until(&mut harness, |h| h.count("[*|aria-label=Battery]") == 1);
    assert_settles_to_zero_frames(&mut harness);
    assert_eq!(
        harness.count("[*|aria-label=Battery] > .ds-icon-nudge > .ds-status-glyph"),
        1,
        "the battery's glyph sits in its nudge"
    );
    assert_eq!(
        harness.count(".ds-icon-nudge"),
        1,
        "no other item carries one"
    );
    // (name, fed, nudges)
    let steps = [
        ("still above", at(25, Charge::Discharging), false),
        ("crossing into low", at(20, Charge::Discharging), true),
        ("lower still", at(15, Charge::Discharging), false),
        ("a wobble back over", at(21, Charge::Discharging), false),
        ("and under again", at(19, Charge::Discharging), false),
        ("plugged in", at(19, Charge::Charging), false),
        ("unplugged, low", at(19, Charge::Discharging), true),
    ];
    for (name, battery, want) in steps {
        assert_eq!(feed(&mut harness, &feeds, battery), want, "{name}");
    }
}
