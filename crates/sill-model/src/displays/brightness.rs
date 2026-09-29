//! Where a brightness change goes, and what one key press moves it to. Pure: the routing
//! ([`route`], [`targets`]) and the 16-step ladder ([`stepped`]).

use serde::{Deserialize, Serialize};

use super::drm::Placement;
use super::policy::KeysTarget;
use crate::ids::{DdcId, OutputName};
use crate::units::Percent;
use crate::volume::Step;

/// A backlight device: the name under `/sys/class/backlight` (`intel_backlight`,
/// `amdgpu_bl1`), which is also what logind's `SetBrightness` takes.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct BacklightName(pub String);

/// The kernel's kind of backlight interface (`/sys/class/backlight/*/type`), in the order a
/// desktop prefers them: firmware (ACPI) over platform over the raw GPU register.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BacklightKind {
    Firmware,
    Platform,
    Raw,
}

impl BacklightKind {
    /// The sysfs `type` word; `None` for one the kernel does not define.
    pub fn parse(word: &str) -> Option<BacklightKind> {
        match word.trim() {
            "firmware" => Some(BacklightKind::Firmware),
            "platform" => Some(BacklightKind::Platform),
            "raw" => Some(BacklightKind::Raw),
            _ => None,
        }
    }
}

/// One backlight and its level.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Backlight {
    pub name: BacklightName,
    pub kind: BacklightKind,
    pub value: u32,
    pub max: u32,
}

/// What a brightness change on one output drives.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum BrightnessTarget {
    /// A built-in panel's backlight, through logind.
    Backlight(BacklightName),
    /// An external monitor's own brightness, over DDC/CI.
    Ddc(DdcId),
    /// Nothing sill can change.
    None,
}

/// The target for the output `name`: a built-in panel's backlight (the preferred kind when
/// there are several), an external monitor's DDC display when one answers on its connector,
/// otherwise nothing.
pub fn route(
    name: &OutputName,
    placement: Placement,
    backlights: &[Backlight],
    ddc: &[(DdcId, Option<OutputName>)],
) -> BrightnessTarget {
    match placement {
        Placement::Internal => backlights
            .iter()
            .min_by_key(|b| b.kind)
            .map_or(BrightnessTarget::None, |b| {
                BrightnessTarget::Backlight(b.name.clone())
            }),
        Placement::External => ddc
            .iter()
            .find(|(_, connector)| connector.as_ref() == Some(name))
            .map_or(BrightnessTarget::None, |(id, _)| BrightnessTarget::Ddc(*id)),
    }
}

/// The outputs a key press acts on: every one, or the one the pointer (else focus) is on,
/// else the first. `outputs` in the compositor's order.
pub fn targets<'a>(
    keys: KeysTarget,
    pointer_or_focus: Option<&OutputName>,
    outputs: &'a [OutputName],
) -> Vec<&'a OutputName> {
    match keys {
        KeysTarget::AllOutputs => outputs.iter().collect(),
        KeysTarget::PointerOutput => pointer_or_focus
            .and_then(|want| outputs.iter().find(|o| *o == want))
            .or_else(|| outputs.first())
            .into_iter()
            .collect(),
    }
}

/// How a brightness (or volume) level changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum LevelChange {
    /// One key press.
    Step(Step),
    /// A slider.
    Set(Percent),
}

/// The ladder has 16 steps, as on a Mac keyboard.
const STEPS: u16 = 16;

/// The percentage of ladder step `k` (0..=16), rounded.
fn rung(k: u16) -> u16 {
    (k * 100 + STEPS / 2) / STEPS
}

/// The level after `change` from `level` (0..=100): a key press moves to the next rung of the
/// ladder in its direction (so an odd level set by a slider joins the ladder on the next press),
/// a slider sets the level outright.
pub fn stepped(level: Percent, change: LevelChange) -> Percent {
    let level = level.0.min(100);
    Percent(match change {
        LevelChange::Set(to) => to.0.min(100),
        LevelChange::Step(Step::Up) => (0..=STEPS).map(rung).find(|r| *r > level).unwrap_or(100),
        LevelChange::Step(Step::Down) => (0..=STEPS)
            .rev()
            .map(rung)
            .find(|r| *r < level)
            .unwrap_or(0),
    })
}

/// A percentage of a device range `0..=max`, rounded; and back.
pub fn percent_of(value: u32, max: u32) -> Percent {
    match max {
        0 => Percent(0),
        max => Percent(
            u16::try_from((u64::from(value.min(max)) * 100 + u64::from(max) / 2) / u64::from(max))
                .unwrap_or(100),
        ),
    }
}

/// The device value for `percent` of `0..=max`, rounded.
pub fn value_of(percent: Percent, max: u32) -> u32 {
    let v = (u64::from(percent.0.min(100)) * u64::from(max) + 50) / 100;
    u32::try_from(v).unwrap_or(max)
}

/// A backlight never goes fully dark from a key press or slider: 0% is its lowest non-zero
/// value, since several panels switch off at 0 and a black laptop screen reads as broken.
pub fn backlight_value(percent: Percent, max: u32) -> u32 {
    value_of(percent, max).max(u32::from(max > 0))
}

#[cfg(test)]
mod tests;
