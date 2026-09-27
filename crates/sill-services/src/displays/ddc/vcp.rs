//! The VCP features sill uses (MCCS 2.2 codes), their raw readings, and a display's controls
//! as typed values. Pure.

use serde::{Deserialize, Serialize};

use crate::volume::Mute;

/// One VCP feature.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Feature {
    Brightness,
    Contrast,
    ColorPreset,
    RedGain,
    GreenGain,
    BlueGain,
    InputSource,
    SpeakerVolume,
    AudioMute,
}

impl Feature {
    /// Every feature, in code order: what a probe reads.
    pub const ALL: [Feature; 9] = [
        Feature::Brightness,
        Feature::Contrast,
        Feature::ColorPreset,
        Feature::RedGain,
        Feature::GreenGain,
        Feature::BlueGain,
        Feature::InputSource,
        Feature::SpeakerVolume,
        Feature::AudioMute,
    ];

    /// The MCCS VCP code.
    pub fn code(self) -> u8 {
        match self {
            Feature::Brightness => 0x10,
            Feature::Contrast => 0x12,
            Feature::ColorPreset => 0x14,
            Feature::RedGain => 0x16,
            Feature::GreenGain => 0x18,
            Feature::BlueGain => 0x1A,
            Feature::InputSource => 0x60,
            Feature::SpeakerVolume => 0x62,
            Feature::AudioMute => 0x8D,
        }
    }
}

/// A raw VCP reading: the current value and the monitor's maximum for it (for a non-continuous
/// feature such as the input source, the maximum means nothing).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct Level {
    pub value: u16,
    pub max: u16,
}

/// A monitor input, by its MCCS value (`0x11` is HDMI 1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct InputSource(pub u16);

impl InputSource {
    /// The name MCCS gives the input, `None` for a vendor-specific one.
    pub fn name(self) -> Option<&'static str> {
        Some(match self.0 & 0xFF {
            0x01 => "VGA 1",
            0x02 => "VGA 2",
            0x03 => "DVI 1",
            0x04 => "DVI 2",
            0x0F => "DisplayPort 1",
            0x10 => "DisplayPort 2",
            0x11 => "HDMI 1",
            0x12 => "HDMI 2",
            0x1B => "USB-C",
            _ => return None,
        })
    }
}

/// A colour preset (VCP 0x14), by its MCCS value (`0x05` is 6500 K, `0x0B` user 1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ColorPreset(pub u16);

/// The three video gains.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct RgbGain {
    pub red: Level,
    pub green: Level,
    pub blue: Level,
}

/// What a display lets sill change, and the values last read or written. `None` is a feature
/// the display did not answer for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct Controls {
    pub brightness: Option<Level>,
    pub contrast: Option<Level>,
    pub color_preset: Option<ColorPreset>,
    pub gain: Option<RgbGain>,
    pub input: Option<InputSource>,
    pub volume: Option<Level>,
    pub mute: Option<Mute>,
}

/// VCP 0x8D's values: 1 mutes, 2 unmutes.
const MUTED: u16 = 1;
const AUDIBLE: u16 = 2;

/// The value to write to VCP 0x8D for `mute`.
pub fn mute_value(mute: Mute) -> u16 {
    match mute {
        Mute::Muted => MUTED,
        Mute::Audible => AUDIBLE,
    }
}

impl Controls {
    /// The controls from one reading per feature (`None` where the read failed). The RGB gain is
    /// only a control when all three channels answered.
    pub fn from_readings(readings: &[(Feature, Option<Level>)]) -> Controls {
        readings
            .iter()
            .fold(
                Controls::default(),
                |controls, (feature, reading)| match reading {
                    Some(level) => controls.read(*feature, *level),
                    None => controls,
                },
            )
            .complete_gain()
    }

    /// `self` after `feature` was read again as `level` (a change made on the monitor). One
    /// channel read again never makes up an RGB gain the display did not have.
    pub fn reread(self, feature: Feature, level: Level) -> Controls {
        let before = self;
        let next = self.read(feature, level);
        match (before.gain, next.gain) {
            (None, Some(_)) => Controls { gain: None, ..next },
            _ => next,
        }
    }

    /// `self` after `feature` read `level` (the partial RGB gain is completed by
    /// [`Controls::from_readings`]).
    fn read(self, feature: Feature, level: Level) -> Controls {
        let gain = self.gain.unwrap_or_default();
        match feature {
            Feature::Brightness => Controls {
                brightness: Some(level),
                ..self
            },
            Feature::Contrast => Controls {
                contrast: Some(level),
                ..self
            },
            Feature::ColorPreset => Controls {
                color_preset: Some(ColorPreset(level.value)),
                ..self
            },
            Feature::RedGain => Controls {
                gain: Some(RgbGain { red: level, ..gain }),
                ..self
            },
            Feature::GreenGain => Controls {
                gain: Some(RgbGain {
                    green: level,
                    ..gain
                }),
                ..self
            },
            Feature::BlueGain => Controls {
                gain: Some(RgbGain {
                    blue: level,
                    ..gain
                }),
                ..self
            },
            Feature::InputSource => Controls {
                input: Some(InputSource(level.value)),
                ..self
            },
            Feature::SpeakerVolume => Controls {
                volume: Some(level),
                ..self
            },
            Feature::AudioMute => Controls {
                mute: match level.value & 0xFF {
                    MUTED => Some(Mute::Muted),
                    AUDIBLE => Some(Mute::Audible),
                    _ => None,
                },
                ..self
            },
        }
    }

    /// A gain with any channel's maximum still zero was not read whole: no gain control.
    fn complete_gain(self) -> Controls {
        let whole = self
            .gain
            .filter(|g| g.red.max > 0 && g.green.max > 0 && g.blue.max > 0);
        Controls {
            gain: whole,
            ..self
        }
    }

    /// `self` after `value` was written to `feature`: the value changes, the maximum stays. A
    /// write to a feature the display never answered for changes nothing.
    pub fn written(self, feature: Feature, value: u16) -> Controls {
        let level = |old: Option<Level>| old.map(|l| Level { value, ..l });
        let gain = |set: fn(RgbGain, u16) -> RgbGain| self.gain.map(|g| set(g, value));
        match feature {
            Feature::Brightness => Controls {
                brightness: level(self.brightness),
                ..self
            },
            Feature::Contrast => Controls {
                contrast: level(self.contrast),
                ..self
            },
            Feature::SpeakerVolume => Controls {
                volume: level(self.volume),
                ..self
            },
            Feature::RedGain => Controls {
                gain: gain(|g, v| RgbGain {
                    red: Level { value: v, ..g.red },
                    ..g
                }),
                ..self
            },
            Feature::GreenGain => Controls {
                gain: gain(|g, v| RgbGain {
                    green: Level {
                        value: v,
                        ..g.green
                    },
                    ..g
                }),
                ..self
            },
            Feature::BlueGain => Controls {
                gain: gain(|g, v| RgbGain {
                    blue: Level { value: v, ..g.blue },
                    ..g
                }),
                ..self
            },
            Feature::ColorPreset => Controls {
                color_preset: self.color_preset.map(|_| ColorPreset(value)),
                ..self
            },
            Feature::InputSource => Controls {
                input: self.input.map(|_| InputSource(value)),
                ..self
            },
            Feature::AudioMute if self.mute.is_some() => {
                self.read(feature, Level { value, max: 0 })
            }
            Feature::AudioMute => self,
        }
    }

    /// Whether the display answered for anything at all.
    pub fn any(&self) -> bool {
        *self != Controls::default()
    }
}

#[cfg(test)]
mod tests;
