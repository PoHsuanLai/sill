//! The volume state's pure transition.

use super::{Step, Volume, VolumeEvent};
use crate::units::{Percent, Presence};

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
