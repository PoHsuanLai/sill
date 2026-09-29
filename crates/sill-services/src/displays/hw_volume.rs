//! A monitor's own speakers following the software volume. PipeWire (through its pulse
//! server) names an HDMI or DisplayPort sink after the monitor at the other end, from the
//! monitor's ELD: `... Digital Stereo (HDMI) [BenQ MA270U]`. When that bracketed name is a DDC
//! display's EDID product name and the display has a speaker volume (VCP 0x62), a change the
//! person makes to the software level or mute is sent on to the monitor as well. Pure.
//!
//! Only changes are followed, never a state: starting sill, or switching the default sink to
//! the monitor, sends nothing, so the monitor keeps whatever volume it had until the person
//! actually changes it. The software path is untouched either way.

use super::brightness::value_of;
use super::ddc::{DdcDisplay, Feature, Write, mute_value};
use super::policy::HardwareVolume;
use sill_model::units::Presence;
use sill_model::volume::Volume;

/// The monitor name a sink description ends with, in square brackets.
pub fn monitor_in_sink(description: &str) -> Option<&str> {
    let inner = description.trim_end().strip_suffix(']')?;
    let (_, name) = inner.rsplit_once('[')?;
    let name = name.trim();
    (!name.is_empty()).then_some(name)
}

/// The DDC display whose product name is the sink's monitor, if it has a speaker volume.
pub fn monitor_for_sink<'a>(sink: &str, ddc: &'a [DdcDisplay]) -> Option<&'a DdcDisplay> {
    let name = monitor_in_sink(sink)?;
    ddc.iter()
        .find(|d| d.endpoint.product == name && d.controls.volume.is_some())
}

/// The writes that carry the change from `before` to `after` to the monitor.
pub fn follow(
    policy: HardwareVolume,
    before: &Volume,
    after: &Volume,
    ddc: &[DdcDisplay],
) -> Vec<Write> {
    let same_sink = before.presence == Presence::Present
        && after.presence == Presence::Present
        && before.sink == after.sink;
    let monitor = match (policy, same_sink) {
        (HardwareVolume::WhenMonitorIsOutput, true) => monitor_for_sink(&after.sink, ddc),
        _ => None,
    };
    let Some(monitor) = monitor else {
        return Vec::new();
    };
    let id = monitor.endpoint.id;
    let level = monitor.controls.volume.map(|l| Write {
        id,
        feature: Feature::SpeakerVolume,
        value: u16::try_from(value_of(after.level, u32::from(l.max))).unwrap_or(l.max),
    });
    let mute = monitor.controls.mute.map(|_| Write {
        id,
        feature: Feature::AudioMute,
        value: mute_value(after.mute),
    });
    [
        level.filter(|_| before.level != after.level),
        mute.filter(|_| before.mute != after.mute),
    ]
    .into_iter()
    .flatten()
    .collect()
}

#[cfg(test)]
mod tests;
