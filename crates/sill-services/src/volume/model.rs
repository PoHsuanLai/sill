//! The volume's state, its commands and its one reducer.

use serde::{Deserialize, Serialize};

use crate::units::{Percent, Presence};

/// The default sink, as the bar's volume item and the OSD draw it.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Volume {
    /// Whether there is a default sink at all.
    pub presence: Presence,
    /// The sink's volume, averaged over channels; can exceed 100.
    pub level: Percent,
    pub mute: Mute,
    /// The sink's human description ("Built-in Audio Analog Stereo").
    pub sink: String,
}

/// Whether the sink is muted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Mute {
    #[default]
    Audible,
    Muted,
}

/// A volume key's direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Step {
    Up,
    Down,
}

/// What the volume service can be told.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum VolumeCommand {
    SetLevel(Percent),
    /// One key press: 5 points, clamped to 0..=100 (the OSD's step).
    Step(Step),
    SetMute(Mute),
    ToggleMute,
}

/// One change the pulse server reported.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum VolumeEvent {
    /// The default sink is now this one (or it changed wholesale).
    Sink {
        sink: String,
        level: Percent,
        mute: Mute,
    },
    Level(Percent),
    Mute(Mute),
    /// There is no default sink any more.
    SinkGone,
    /// The server went away.
    ServerGone,
}

/// `volume` after `event`. A level or mute change with no sink is dropped: there is nothing
/// for it to describe.
pub fn step(volume: Volume, event: VolumeEvent) -> Volume {
    match (volume.presence, event) {
        (_, VolumeEvent::Sink { sink, level, mute }) => Volume {
            presence: Presence::Present,
            level,
            mute,
            sink,
        },
        (Presence::Present, VolumeEvent::Level(level)) => Volume { level, ..volume },
        (Presence::Present, VolumeEvent::Mute(mute)) => Volume { mute, ..volume },
        (Presence::Absent, VolumeEvent::Level(_) | VolumeEvent::Mute(_)) => volume,
        (_, VolumeEvent::SinkGone | VolumeEvent::ServerGone) => Volume::default(),
    }
}

/// The level one key press moves to: 5 points, clamped to 0..=100 (a level already above 100,
/// set elsewhere, steps down normally and never steps further up).
pub fn stepped(level: Percent, step: Step) -> Percent {
    match step {
        Step::Up if level.0 >= 100 => level,
        Step::Up => Percent((level.0 + 5).min(100)),
        Step::Down => Percent(level.0.saturating_sub(5)),
    }
}

#[cfg(test)]
mod tests {
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
}
