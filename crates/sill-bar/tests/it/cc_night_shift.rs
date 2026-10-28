//! The Night Shift row under the Display module on the real control center: absent while
//! casement's settings are out of reach, a switch that asks for "until tomorrow" or off, a line
//! that says what Night Shift is doing, and the warmth slider while it warms or will. Fake
//! services on `Clock::Virtual`; nothing here writes casement's file.

use crate::cc_support;

use cc_support::details::{center, ms, press};

use sill_model::night_shift::{
    Backend, Manual, NightLight, NightShift, NightShiftCommand, Schedule, UnixSeconds, Warmth,
};

const ROW: &str = "#night-shift";
const SWITCH: &str = "#night-shift .ds-toggle";
const SLIDER: &str = "#night-shift .ds-slider";
const RAIL: &str = "#night-shift .ds-slider-track";

fn ready(schedule: Schedule, manual: Option<Manual>, backend: Backend) -> NightShift {
    NightShift {
        reached: true,
        light: NightLight {
            schedule,
            ..NightLight::default()
        },
        manual,
        backend,
    }
}

fn by_hand() -> Manual {
    Manual {
        until: UnixSeconds(1),
        restore: NightLight::default(),
    }
}

#[test]
fn nothing_is_drawn_while_casements_settings_cannot_be_reached() {
    let running = center(|_| {});
    assert_eq!(running.harness.count(ROW), 0);
}

#[test]
fn the_switch_asks_for_until_tomorrow_when_off_and_for_off_when_on() {
    let mut running = center(|feeds| {
        feeds
            .night_shift
            .send_replace(ready(Schedule::Off, None, Backend::Software));
    });
    assert_eq!(running.harness.count(ROW), 1);
    assert_eq!(
        running.harness.count(SLIDER),
        0,
        "no warmth to set while off"
    );
    assert!(running.harness.html().contains("Night Shift"));
    press(&mut running.harness, SWITCH);
    running.harness.advance(ms(100));
    assert_eq!(
        running.feeds.night_shift_commands.try_recv().ok(),
        Some(NightShiftCommand::TurnOnUntilTomorrow)
    );

    running.feeds.night_shift.send_replace(ready(
        Schedule::Always,
        Some(by_hand()),
        Backend::Software,
    ));
    running.harness.advance(ms(100));
    assert!(running.harness.html().contains("On until tomorrow"));
    assert_eq!(
        running.harness.count(SLIDER),
        1,
        "the warmth while it warms"
    );
    press(&mut running.harness, SWITCH);
    running.harness.advance(ms(100));
    assert_eq!(
        running.feeds.night_shift_commands.try_recv().ok(),
        Some(NightShiftCommand::TurnOff)
    );
}

#[test]
fn a_press_on_the_warmth_slider_sets_the_warmth_between_less_and_more_warm() {
    let mut running = center(|feeds| {
        feeds
            .night_shift
            .send_replace(ready(Schedule::Always, None, Backend::Software));
    });
    running.harness.advance(ms(100));
    let Some(rail) = running.harness.centre(RAIL) else {
        panic!("the slider is not laid out:\n{}", running.harness.html());
    };
    running.harness.send(ds_harness::Input::click(rail));
    running.harness.advance(ms(100));
    let Some(NightShiftCommand::SetWarmth(warmth)) =
        running.feeds.night_shift_commands.try_recv().ok()
    else {
        panic!("the slider sent no warmth:\n{}", running.harness.html());
    };
    let middle = Warmth::clamped(4600).kelvin();
    assert!(
        warmth.kelvin().abs_diff(middle) < 150,
        "the middle of the slider is about {middle} K, not {}",
        warmth.kelvin()
    );
}
