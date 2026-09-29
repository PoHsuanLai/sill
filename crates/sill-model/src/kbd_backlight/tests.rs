use super::{KbdBacklight, KbdEvent, KbdLevel, step, step_for};
use crate::units::{Percent, Presence};

fn at(value: u32, max: u32) -> KbdLevel {
    KbdLevel { value, max }
}

fn present(value: u32, max: u32) -> KbdBacklight {
    KbdBacklight {
        presence: Presence::Present,
        level: at(value, max),
    }
}

#[test]
fn the_backlight_follows_upower() {
    // (name, before, event, after)
    let cases = [
        (
            "found",
            KbdBacklight::default(),
            KbdEvent::Found(at(1, 3)),
            present(1, 3),
        ),
        (
            "a key press",
            present(1, 3),
            KbdEvent::Brightness(2),
            present(2, 3),
        ),
        (
            "past the range is the top",
            present(1, 3),
            KbdEvent::Brightness(9),
            present(3, 3),
        ),
        (
            "a step before any backlight",
            KbdBacklight::default(),
            KbdEvent::Brightness(2),
            KbdBacklight::default(),
        ),
        (
            "gone",
            present(2, 3),
            KbdEvent::Gone,
            KbdBacklight::default(),
        ),
    ];
    for (name, before, event, after) in cases {
        assert_eq!(step(before, event), after, "{name}");
    }
}

#[test]
fn steps_and_percentages_round_to_the_nearest() {
    // (value, max, percent)
    let shown = [
        (0, 3, 0),
        (1, 3, 33),
        (2, 3, 67),
        (3, 3, 100),
        (1, 2, 50),
        (0, 0, 0),
    ];
    for (value, max, percent) in shown {
        assert_eq!(at(value, max).percent(), Percent(percent), "{value}/{max}");
    }
    // (percent, max, step)
    let set = [
        (0, 3, 0),
        (20, 3, 1),
        (40, 3, 1),
        (50, 3, 2),
        (100, 3, 3),
        (70, 2, 1),
        (80, 2, 2),
        (150, 3, 3),
    ];
    for (percent, max, step) in set {
        assert_eq!(step_for(Percent(percent), max), step, "{percent}% of {max}");
    }
}
