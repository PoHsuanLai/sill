use super::{Mute, Step, Volume, VolumeEvent, step, stepped};
use crate::units::{Percent, Presence};

fn sink(level: u16, mute: Mute) -> Volume {
    Volume {
        presence: Presence::Present,
        level: Percent(level),
        mute,
        sink: "Speakers".to_owned(),
    }
}

#[test]
fn events_reduce_to_the_expected_volume() {
    let cases: Vec<(&str, Volume, VolumeEvent, Volume)> = vec![
        (
            "a sink appears",
            Volume::default(),
            VolumeEvent::Sink {
                sink: "Speakers".to_owned(),
                level: Percent(40),
                mute: Mute::Audible,
            },
            sink(40, Mute::Audible),
        ),
        (
            "level",
            sink(40, Mute::Audible),
            VolumeEvent::Level(Percent(120)),
            sink(120, Mute::Audible),
        ),
        (
            "mute",
            sink(40, Mute::Audible),
            VolumeEvent::Mute(Mute::Muted),
            sink(40, Mute::Muted),
        ),
        (
            "a level with no sink is dropped",
            Volume::default(),
            VolumeEvent::Level(Percent(40)),
            Volume::default(),
        ),
        (
            "a mute with no sink is dropped",
            Volume::default(),
            VolumeEvent::Mute(Mute::Muted),
            Volume::default(),
        ),
        (
            "sink gone",
            sink(40, Mute::Muted),
            VolumeEvent::SinkGone,
            Volume::default(),
        ),
        (
            "server gone",
            sink(40, Mute::Muted),
            VolumeEvent::ServerGone,
            Volume::default(),
        ),
    ];
    for (name, before, event, want) in cases {
        assert_eq!(step(before, event), want, "{name}");
    }
}

#[test]
fn a_key_press_steps_five_points_within_bounds() {
    const CASES: &[(u16, Step, u16)] = &[
        (40, Step::Up, 45),
        (98, Step::Up, 100),
        (100, Step::Up, 100),
        (130, Step::Up, 130),
        (40, Step::Down, 35),
        (3, Step::Down, 0),
        (130, Step::Down, 125),
    ];
    for &(level, step, want) in CASES {
        assert_eq!(
            stepped(Percent(level), step),
            Percent(want),
            "{level} {step:?}"
        );
    }
}
