//! What a brightness change and the night-warmth setting turn into, as values: DDC writes for
//! the worker and backlight values for logind. Pure.

use std::collections::BTreeMap;

use super::brightness::{
    BacklightName, BrightnessTarget, LevelChange, backlight_value, stepped, targets, value_of,
};
use super::ddc::{DdcId, Feature, Level, RgbGain, Write};
use super::model::Displays;
use super::policy::{KeysTarget, NightWarmth};
use super::warmth::{scaled, warmth_gains};
use crate::cosmic_wl::OutputName;
use crate::units::Percent;

/// One effect of a brightness change.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BrightnessAction {
    Ddc(Write),
    Backlight { name: BacklightName, value: u32 },
}

/// The actions for `change` on the outputs `keys` picks (`at` being the pointer's or the
/// focused output). Each target moves from its own level, and two outputs sharing a target (a
/// mirrored pair) move it once.
pub fn brightness_actions(
    displays: &Displays,
    keys: KeysTarget,
    at: Option<&OutputName>,
    change: LevelChange,
) -> Vec<BrightnessAction> {
    let names: Vec<OutputName> = displays.outputs.iter().map(|o| o.name.clone()).collect();
    let mut chosen: Vec<BrightnessTarget> = Vec::new();
    for target in targets(keys, at, &names)
        .into_iter()
        .map(|name| displays.target(name))
    {
        if target != BrightnessTarget::None && !chosen.contains(&target) {
            chosen.push(target);
        }
    }
    chosen
        .iter()
        .filter_map(|target| action(displays, target, change))
        .collect()
}

fn action(
    displays: &Displays,
    target: &BrightnessTarget,
    change: LevelChange,
) -> Option<BrightnessAction> {
    let now = displays.brightness(target)?;
    let next = stepped(now, change);
    match target {
        BrightnessTarget::Backlight(name) => {
            let max = displays.backlights.iter().find(|b| &b.name == name)?.max;
            Some(BrightnessAction::Backlight {
                name: name.clone(),
                value: backlight_value(next, max),
            })
        }
        BrightnessTarget::Ddc(id) => {
            let level = displays
                .ddc
                .iter()
                .find(|d| d.endpoint.id == *id)?
                .controls
                .brightness?;
            Some(BrightnessAction::Ddc(Write {
                id: *id,
                feature: Feature::Brightness,
                value: u16::try_from(value_of(next, u32::from(level.max))).unwrap_or(level.max),
            }))
        }
        BrightnessTarget::None => None,
    }
}

/// Each warmed display's gains as they were before sill touched them.
pub type Originals = BTreeMap<DdcId, RgbGain>;

/// The writes that bring every DDC display with RGB gains to `warmth`, and the originals to
/// keep: `Hardware` scales each display's own gains (remembered the first time) by the warmth
/// multipliers; `Off` puts back every gain sill changed and forgets them.
pub fn warmth_writes(
    displays: &Displays,
    originals: &Originals,
    warmth: NightWarmth,
) -> (Originals, Vec<Write>) {
    match warmth {
        NightWarmth::Off => {
            let writes = originals
                .iter()
                .flat_map(|(id, original)| gain_writes(displays, *id, *original))
                .collect();
            (Originals::new(), writes)
        }
        NightWarmth::Hardware(strength) => warm(displays, originals, strength),
    }
}

fn warm(displays: &Displays, originals: &Originals, strength: Percent) -> (Originals, Vec<Write>) {
    let factor = warmth_gains(strength);
    let kept: Originals = displays
        .ddc
        .iter()
        .filter_map(|d| {
            let current = d.controls.gain?;
            Some((
                d.endpoint.id,
                originals.get(&d.endpoint.id).copied().unwrap_or(current),
            ))
        })
        .collect();
    let writes = kept
        .iter()
        .flat_map(|(id, original)| {
            let target = RgbGain {
                red: Level {
                    value: scaled(original.red.value, factor.red),
                    ..original.red
                },
                green: Level {
                    value: scaled(original.green.value, factor.green),
                    ..original.green
                },
                blue: Level {
                    value: scaled(original.blue.value, factor.blue),
                    ..original.blue
                },
            };
            gain_writes(displays, *id, target)
        })
        .collect();
    (kept, writes)
}

/// The writes that take display `id`'s gains to `target`, channel by channel, skipping any
/// already there.
fn gain_writes(displays: &Displays, id: DdcId, target: RgbGain) -> Vec<Write> {
    let Some(current) = displays
        .ddc
        .iter()
        .find(|d| d.endpoint.id == id)
        .and_then(|d| d.controls.gain)
    else {
        return Vec::new();
    };
    [
        (Feature::RedGain, current.red.value, target.red.value),
        (Feature::GreenGain, current.green.value, target.green.value),
        (Feature::BlueGain, current.blue.value, target.blue.value),
    ]
    .into_iter()
    .filter(|(_, now, want)| now != want)
    .map(|(feature, _, value)| Write { id, feature, value })
    .collect()
}

#[cfg(test)]
mod tests;
